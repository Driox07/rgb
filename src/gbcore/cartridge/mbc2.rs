use super::MBC;

#[derive(Debug)]
pub struct MBC2 {
    ram_enabled: bool,
    rom_bank: u8
}

impl MBC2 {
    pub fn new() -> MBC2 {
        MBC2 {
            ram_enabled: false,
            rom_bank: 1
        }
    }
}

impl MBC for MBC2 {
    fn mbc_info(&self) -> String {
        "MBC2".to_string()
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
            Some(addr as usize - 0xA000)
        }
    }

    fn ram_read(&self, value: u8) -> u8 {
        value | 0xF0
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            ..=0x3FFF => {
                if addr & 0x0100 == 0 {
                    self.ram_enabled = value & 0x0F == 0xA;
                } else {
                    self.rom_bank = value & 0x0F;
                    if self.rom_bank == 0 { self.rom_bank = 1; }
                }
            },

            _ => ()
        }
    }
}