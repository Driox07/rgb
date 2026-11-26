use super::MBC;

#[derive(Debug)]
pub struct MBC5 {
    ram_enabled: bool,
    rom_bank: u16,
    ram_bank: u8
}

impl MBC5 {
    pub fn new() -> MBC5 {
        MBC5 {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0
        }
    }
}

impl MBC for MBC5 {
    fn mbc_info(&self) -> String {
        "MBC5".to_string()
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
            Some(self.ram_bank as usize * 0x2000 + (addr as usize - 0xA000))
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            ..=0x1FFF => self.ram_enabled = value & 0x0F == 0x0A,

            0x2000..=0x2FFF => {
                self.rom_bank = (self.rom_bank & 0x0100) | value as u16;
            },

            0x3000..=0x3FFF => {
                self.rom_bank = (self.rom_bank & 0x00FF) | (value as u16 & 1) << 8;
            },

            0x4000..=0x5FFF => {
                self.ram_bank = value & 0x0F;
            },

            _ => ()
        }
    }
}