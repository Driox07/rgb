const BANK0_END: u16 = 0x3FFF;
const BANK1_BEGIN: u16 = 0x4000;
const BANK1_END: u16 = 0x7FFF;
const ERAM_BEGIN: u16 = 0xA000;
const ERAM_END:   u16 = 0xBFFF;

const MIN_LENGTH: u16 = 0x150;

const TITLE_BEGIN:   u16 = 0x0134;
const TITLE_END:     u16 = 0x0143;
const TYPE_ADDR:     u16 = 0x0147;
const RAM_TYPE_ADDR: u16 = 0x0149;
const CHECKSUM_END:  u16 = 0x014C;
const CHECKSUM_ADDR: u16 = 0x014D;

const ERAM_NONE: u8 = 0;
const ERAM_8KB: u8 = 2;
const ERAM_32KB: u8 = 3;

#[derive(Debug)]
pub struct Cartridge{
    pub title: String,
    rom: Vec<u8>,
    ram: Option<Vec<u8>>,
    ram_enabled: bool,
    ram_written: bool,
    selected_bank: u8,
    bank2: u8,
    mode: u8,
    mbc: u8,
    has_battery: bool
}

impl Cartridge{

    pub fn new(rom: Vec<u8>) -> Result<Cartridge, String> {

        if rom.len() < MIN_LENGTH as usize {
            return Err("Invalid ROM format".to_string());
        }

        let expected_checksum: u8 = rom.get(CHECKSUM_ADDR as usize).copied().unwrap_or(0xFF);
        let mut calculated_checksum: u8 = 0;

        for i in TITLE_BEGIN as usize ..= CHECKSUM_END as usize {
            let b = rom.get(i as usize).copied().unwrap();
            calculated_checksum = calculated_checksum.wrapping_sub(b).wrapping_sub(1);
        }

        if expected_checksum != calculated_checksum {
            return Err("Invalid checksum".to_string());
        }

        let mbc = rom.get(TYPE_ADDR as usize).copied().unwrap();
        if mbc > 3 {
            println!("This ROM needs MBC {:#04X} but it's not implemented yet. Expect bad things XD", mbc);
        }

        let has_battery = match mbc {
           3 => true,
           _ => false
        };

        let ram = match rom.get(RAM_TYPE_ADDR as usize).copied().unwrap() {
            ERAM_NONE => None,
            ERAM_8KB => Some(vec![0; 0x2000]),
            ERAM_32KB => Some(vec![0; 0x8000]),
            weirdo => {
                print!("Extra RAM type {:#04X} not recognized. Expect bad things XD", weirdo);
                None
            }
        };
        
        let bytes: &[u8] = &rom[TITLE_BEGIN as usize ..= TITLE_END as usize];
        let text = String::from_utf8_lossy(bytes);
        
        Ok(Cartridge { 
            title: text.trim_end_matches("\0").to_string(), 
            rom,
            ram,
            ram_enabled: false,
            ram_written: false,
            selected_bank: 1,
            bank2: 0,
            mode: 0,
            mbc,
            has_battery
        })
    }

    pub fn has_battery(&self) -> bool {
        self.has_battery
    }

    pub fn get_ram(&self) -> Option<&[u8]> {
        match &self.ram {
            None => None,
            Some(ram) => Some(ram.as_slice())
        }
    }

    pub fn ram_written(&mut self) -> bool {
        let rw = self.ram_written && self.has_battery;
        self.ram_written = false;
        rw
    }

    pub fn load_ram(&mut self, lram: &[u8]) -> Result<(), String>{
        if !self.has_battery { return Err("This game has no battery.".to_string()); }
        match &mut self.ram {
            None => Err("SAV data is corrupted or not compatible with this game".to_string()),
            Some(ram) => {
                if lram.len() == ram.len() {
                    ram.copy_from_slice(lram);
                    Ok(())
                }else{
                    Err("SAV data is corrupted or not compatible with this game".to_string())
                }
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            ..=BANK0_END => self.rom.get(addr as usize).copied().unwrap_or(0xFF),
            BANK1_BEGIN..=BANK1_END => {
                let bank = (self.bank2 << 5) | self.selected_bank;
                let raddr = bank as usize * 0x4000 + (addr as usize - 0x4000);
                self.rom[raddr % self.rom.len()]
            },
            ERAM_BEGIN..=ERAM_END => {
                match &self.ram {
                    None => 0xFF,
                    Some(ram) => {
                        if !self.ram_enabled { return 0xFF }
                        let bank = if self.mode == 1 { self.bank2 } else {  0 };
                        let raddr = bank as usize * 0x2000 + (addr - ERAM_BEGIN) as usize;
                        ram[raddr % ram.len()]
                    }
                }
            },
            _ => 0xFF
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        if self.mbc == 0x00 { return }
        match addr {
            ..=0x1FFF => self.ram_enabled = (value & 0xF) == 0xA,
            0x2000..=0x3FFF => {
                self.selected_bank = value & 0x1F;
                if self.selected_bank == 0 { self.selected_bank += 1; } 
            },
            0x4000..=0x5FFF => self.bank2 = value & 0x3,
            0x6000..=0x7FFF => self.mode = value & 1,
            ERAM_BEGIN..=ERAM_END => {
                match self.ram.as_mut() {
                    None => (),
                    Some(ram) => {
                        if !self.ram_enabled { return } 
                        self.ram_written = true;
                        let bank = if self.mode == 1 { self.bank2 } else {  0 };
                        let raddr = bank as usize * 0x2000 + (addr - ERAM_BEGIN) as usize;
                        let ram_len = ram.len(); 
                        ram[raddr % ram_len] = value;
                    }
                }
            }
            _ => ()
        }
    }

}

#[cfg(test)]
mod tests {
use std::assert_matches;

use super::*;

    #[test]
    fn bad_checksum() {
        let rom = std::fs::read("./test_roms/bad_checksum.gb");
        assert_matches!(rom, Ok(_));
        let cartridge = Cartridge::new(rom.unwrap());
        assert_matches!(cartridge, Err(_));
        assert_eq!(cartridge.unwrap_err(), "Invalid checksum".to_string());
    }

    #[test]
    fn correct_checksum() {
        let rom = std::fs::read("./test_roms/valid_rom.gb");
        assert_matches!(rom, Ok(_));
        let cartridge = Cartridge::new(rom.unwrap());
        assert_matches!(cartridge, Ok(_));
    }

    #[test]
    fn too_short_rom() {
        let rom = std::fs::read("./test_roms/short_rom.gb");
        assert_matches!(rom, Ok(_));
        let cartridge = Cartridge::new(rom.unwrap());
        assert_matches!(cartridge, Err(_));
        assert_eq!(cartridge.unwrap_err(), "Invalid ROM format".to_string());
    }

    #[test]
    fn minimal_rom() {
        let rom = std::fs::read("./test_roms/minimal_rom.gb");
        assert_matches!(rom, Ok(_));
        let cartridge = Cartridge::new(rom.unwrap());
        assert_matches!(cartridge, Ok(_));
    }

        // ---------- MBC1 ----------

    // Builds an in-memory ROM with `banks` banks of 16 KB.
    // The first byte of each bank holds its bank number, so we can tell which bank is mapped.
    fn make_rom(cart_type: u8, ram_type: u8, banks: usize) -> Cartridge {
        let mut rom = vec![0u8; banks * 0x4000];
        for b in 0..banks {
            rom[b * 0x4000] = b as u8;
        }
        rom[TYPE_ADDR as usize] = cart_type;
        rom[RAM_TYPE_ADDR as usize] = ram_type;

        let mut checksum: u8 = 0;
        for i in TITLE_BEGIN as usize..=CHECKSUM_END as usize {
            checksum = checksum.wrapping_sub(rom[i]).wrapping_sub(1);
        }
        rom[CHECKSUM_ADDR as usize] = checksum;

        Cartridge::new(rom).unwrap()
    }

    #[test]
    fn mbc1_rom_bank_switching() {
        let mut cart = make_rom(0x01, 0x00, 8);

        // Bank 0 always at 0x0000, bank 1 by default at 0x4000
        assert_eq!(cart.read(0x0000), 0);
        assert_eq!(cart.read(0x4000), 1);

        cart.write(0x2000, 3);
        assert_eq!(cart.read(0x4000), 3);
        assert_eq!(cart.read(0x0000), 0);

        // Writing 0 selects bank 1
        cart.write(0x2000, 0);
        assert_eq!(cart.read(0x4000), 1);

        // Bank 31 does not exist in an 8-bank ROM -> wraps to 31 % 8 = 7
        cart.write(0x3FFF, 0x1F);
        assert_eq!(cart.read(0x4000), 7);
    }

    #[test]
    fn mbc1_bank2_upper_rom_bits() {
        let mut cart = make_rom(0x01, 0x00, 64); // 1 MB

        cart.write(0x2000, 2);
        cart.write(0x4000, 1); // bank = (1 << 5) | 2 = 34
        assert_eq!(cart.read(0x4000), 34);
    }

    #[test]
    fn no_mbc_ignores_bank_writes() {
        let mut cart = make_rom(0x00, 0x00, 2); // 32 KB, no MBC

        cart.write(0x2000, 2);
        assert_eq!(cart.read(0x4000), 1);
    }

    #[test]
    fn mbc1_ram() {
        let mut cart = make_rom(0x03, 0x03, 4); // MBC1 + RAM, 32 KB of RAM

        // Disabled by default: reads 0xFF and writes are ignored
        cart.write(0xA000, 0x42);
        assert_eq!(cart.read(0xA000), 0xFF);

        // Enable and use bank 0, including the last address
        cart.write(0x0000, 0x0A);
        cart.write(0xA000, 0x42);
        cart.write(0xBFFF, 0x24);
        assert_eq!(cart.read(0xA000), 0x42);
        assert_eq!(cart.read(0xBFFF), 0x24);

        // Mode 1 + bank2 = 1 -> RAM bank 1 (empty)
        cart.write(0x6000, 1);
        cart.write(0x4000, 1);
        assert_eq!(cart.read(0xA000), 0x00);
        cart.write(0xA000, 0x99);

        // Back to bank 0 -> original data is still there
        cart.write(0x4000, 0);
        assert_eq!(cart.read(0xA000), 0x42);

        // Disable again
        cart.write(0x0000, 0x00);
        assert_eq!(cart.read(0xA000), 0xFF);
    }

}