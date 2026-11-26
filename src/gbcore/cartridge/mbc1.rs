use super::MBC;

#[derive(Debug)]
pub struct MBC1 {
    ram_enabled: bool,
    bank1: u8,
    bank2: u8,
    mode: u8
}

impl MBC1 {
    pub fn new() -> MBC1 {
        MBC1 {
            ram_enabled: false,
            bank1: 1,
            bank2: 0,
            mode: 0
        }
    }
}

impl MBC for MBC1 {
    fn mbc_info(&self) -> String {
        "MBC1".to_string()
    }

    fn rom_offset(&self, addr: u16) -> usize {
        match addr {
            ..=0x3FFF => addr as usize,
            _ => {
                let bank = ((self.bank2 << 5) | self.bank1) as usize;
                bank * 0x4000 + (addr as usize - 0x4000)
            }
        }
    }

    fn ram_offset(&self, addr: u16) -> Option<usize> {
        if !self.ram_enabled { None }
        else {
            let bank = if self.mode == 1 { self.bank2 } else { 0 } as usize;
            Some(bank * 0x2000 + (addr as usize - 0xA000))
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            ..=0x1FFF => self.ram_enabled = value & 0x0F == 0x0A,

            0x2000..=0x3FFF => {
                self.bank1 = value & 0x1F;
                if self.bank1 == 0 { self.bank1 = 1; }
            },

            0x4000..=0x5FFF => self.bank2 = value & 0x03,

            _ => self.mode = value & 1
        }
    }
}