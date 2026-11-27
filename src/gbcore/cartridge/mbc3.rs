use super::MBC;

const TCYCLES_PER_SEC: u32 = 4_194_304;

const RTC_SECONDS: usize = 0;
const RTC_MINUTES: usize = 1;
const RTC_HOURS: usize = 2;
const RTC_DAY_LO: usize = 3;
const RTC_DAY_HI: usize = 4;

const RTC_FLAG_HALT: u8 = 0x40;
const RTC_FLAG_OVERFLOW: u8 = 0x80;

#[derive(Debug)]
pub struct MBC3 {
    ram_enabled: bool,
    rom_bank: u8,
    ram_clk_sel: u8,
    rtc: [u8; 5],
    latch: [u8; 5],
    latch_last: u8,
    cycles: u32
}

impl MBC3 {
    pub fn new() -> MBC3 {
        MBC3 {
            ram_enabled: false,
            rom_bank: 1,
            ram_clk_sel: 0,
            rtc: [0; 5],
            latch: [0; 5],
            latch_last: 0,
            cycles: 0
        }
    }

    fn advance_clock(&mut self) {
        self.rtc[RTC_SECONDS] = self.rtc[RTC_SECONDS].wrapping_add(1);

        if self.rtc[RTC_SECONDS] >= 60 {

            if self.rtc[RTC_SECONDS] == 60{
                self.rtc[RTC_SECONDS] = 0;
                self.rtc[RTC_MINUTES] = self.rtc[RTC_MINUTES].wrapping_add(1);
            }

            if self.rtc[RTC_SECONDS] > 63 {
                self.rtc[RTC_SECONDS] = 0;
            }
        }

        if self.rtc[RTC_MINUTES] >= 60 {
            self.rtc[RTC_MINUTES] = 0;
            self.rtc[RTC_HOURS] = self.rtc[RTC_HOURS].wrapping_add(1);
        }

        if self.rtc[RTC_HOURS] >= 24 {
            self.rtc[RTC_HOURS] = 0;
            let mut curr_days: u16 = (self.rtc[RTC_DAY_HI] as u16 & 0x01) << 8 | self.rtc[RTC_DAY_LO] as u16;
            curr_days += 1;
            if curr_days == 512 {
                curr_days = 0;
                self.rtc[RTC_DAY_HI] = self.rtc[RTC_DAY_HI] | RTC_FLAG_OVERFLOW;
            }
            self.rtc[RTC_DAY_LO] = curr_days as u8;
            self.rtc[RTC_DAY_HI] = (self.rtc[RTC_DAY_HI] & 0xFE) | (curr_days >> 8) as u8 & 0x01;
        }
    }
}

impl MBC for MBC3 {
    fn mbc_info(&self) -> String {
        "MBC3".to_string()
    }

    fn tick(&mut self, t: u8) {
        if self.rtc[RTC_DAY_HI] & RTC_FLAG_HALT != 0 { return }
        self.cycles = self.cycles.wrapping_add(t as u32);
        if self.cycles >= TCYCLES_PER_SEC {
            self.cycles = self.cycles.wrapping_sub(TCYCLES_PER_SEC);
            self.advance_clock();
        }
    }

    fn rom_offset(&self, addr: u16) -> usize {
        match addr {
            ..=0x3FFF => addr as usize,
            _ => {
                self.rom_bank as usize * 0x4000 + (addr as usize - 0x4000)
            }
        }
    }

    fn ram_offset(&self, addr: u16) -> Option<usize> {
        if !self.ram_enabled { None }
        else {
            match self.ram_clk_sel {
                ..=0x03 => Some(self.ram_clk_sel as usize * 0x2000 + (addr as usize - 0xA000)),
                0x08..=0x0C => None,
                _ => None,
            }
        }
    }

    fn rtc_read(&self) -> Option<u8> {
        if !self.ram_enabled { None }
        else {
            match self.ram_clk_sel {
                0x08..=0x0C => Some(self.latch[self.ram_clk_sel as usize - 0x08]),
                _ => None
            }
        }
    }

    fn rtc_write(&mut self, value: u8) -> bool {
        if !self.ram_enabled { false }
        else {
            match self.ram_clk_sel {
                0x08..=0x0C => {
                    self.rtc[self.ram_clk_sel as usize - 0x08] = value;
                    true
                },
                _ => false
            }
        }
    }

    fn rtc_save(&self) -> Vec<u8> {
        let mut rtc_data = Vec::new();
        for reg in self.rtc {
            rtc_data.extend_from_slice(&(reg as u32).to_le_bytes());
        }
        for reg in self.latch {
            rtc_data.extend_from_slice(&(reg as u32).to_le_bytes());
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        rtc_data.extend_from_slice(&now.to_le_bytes());
        rtc_data
    }

    fn rtc_load(&mut self, data: &[u8]) {
        if data.len() < 44 { return }
        for (i, v) in self.rtc.iter_mut().enumerate() {
            let value = u32::from_le_bytes(data[i*4..i*4+4].try_into().unwrap());
            *v = value as u8;
        }
        for(i, v) in self.latch.iter_mut().enumerate() {
            let value = u32::from_le_bytes(data[20 + i*4 .. 20 + i*4+4].try_into().unwrap());
            *v = value as u8;
        }
        let saved_time = if data.len() < 48 {
            u32::from_le_bytes(data[40..44].try_into().unwrap()) as u64
        }else{
            u64::from_le_bytes(data[40..48].try_into().unwrap())
        };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let elapsed = now.saturating_sub(saved_time);
        if self.rtc[RTC_DAY_HI] & RTC_FLAG_HALT == 0 {
            for _ in 0..elapsed {
                self.advance_clock();
            }
        }
    } 

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            ..=0x1FFF => self.ram_enabled = value & 0x0F == 0x0A,

            0x2000..=0x3FFF => {
                self.rom_bank = value & 0x07F;
                if self.rom_bank == 0 { self.rom_bank = 1 };
            },

            0x4000..=0x5FFF => {
                self.ram_clk_sel = value;
            },

            0x6000..=0x7FFF => {
                if self.latch_last == 0x00 && value == 0x01 {
                    self.latch = self.rtc;
                }
                self.latch_last = value;
            }

            _ => ()
        }
    }

}