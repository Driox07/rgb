const ERAM_BEGIN: u16 = 0xA000;
const ERAM_END:   u16 = 0xBFFF;

const MIN_LENGTH: u16 = 0x150;

const TITLE_BEGIN:   u16 = 0x0134;
const TITLE_END:     u16 = 0x0143;
const TYPE_ADDR:     u16 = 0x0147;
const _RAM_TYPE_ADDR: u16 = 0x0149;
const CHECKSUM_END:  u16 = 0x014C;
const CHECKSUM_ADDR: u16 = 0x014D;

#[derive(Debug)]
pub struct Cartridge{
    pub title: String,
    rom: Vec<u8>
}

impl Cartridge{

    pub fn new(r: Vec<u8>) -> Result<Cartridge, String> {

        if r.len() < MIN_LENGTH as usize {
            return Err("Invalid ROM format".to_string());
        }

        let expected_checksum: u8 = r.get(CHECKSUM_ADDR as usize).copied().unwrap_or(0xFF);
        let mut calculated_checksum: u8 = 0;

        for i in TITLE_BEGIN as usize ..= CHECKSUM_END as usize {
            let b = r.get(i as usize).copied().unwrap();
            calculated_checksum = calculated_checksum.wrapping_sub(b).wrapping_sub(1);
        }

        if expected_checksum != calculated_checksum {
            return Err("Invalid checksum".to_string());
        }

        if r.get(TYPE_ADDR as usize).copied().unwrap() != 0x00{
            println!("This ROM needs MBC but it's not implemented yet. Expect bad things XD");
            // TODO: remove this when MCB is implemented
        }
        
        let bytes: &[u8] = &r[TITLE_BEGIN as usize ..= TITLE_END as usize];
        let text = String::from_utf8_lossy(bytes);
        
        Ok(Cartridge { 
            title: text.trim_end_matches("\0").to_string(), 
            rom: r 
        })
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            ..=0x7FFF => self.rom.get(addr as usize).copied().unwrap_or(0xFF),
            ERAM_BEGIN..=ERAM_END => 0xFF,
            _ => 0xFF
        }
    }

    pub fn write(&mut self, _addr: u16, _value: u8) {

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

}