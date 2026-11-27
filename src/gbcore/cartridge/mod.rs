mod no_mbc;
mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;

use no_mbc::NoMBC;
use mbc1::MBC1;
use mbc2::MBC2;
use mbc3::MBC3;
use mbc5::MBC5;

const _BANK0_END: u16 = 0x3FFF;
const _BANK1_BEGIN: u16 = 0x4000;
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

const ERAM_NONE: u8 = 0x00;
const ERAM_8KB: u8 = 0x02;
const ERAM_32KB: u8 = 0x03;
const ERAM_64KB: u8 = 0x05;
const ERAM_128KB: u8 = 0x04;

#[derive(Debug)]
pub struct Cartridge{
    pub title: String,
    rom: Vec<u8>,
    ram: Option<Vec<u8>>,
    ram_written: bool,
    mbc: Box<dyn MBC>,
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

        let ctr_type = rom.get(TYPE_ADDR as usize).copied().unwrap();
        let mbc: Box<dyn MBC> = match ctr_type {
            0x00 => Box::new(NoMBC),
            0x01..=0x03 => Box::new(MBC1::new()),
            0x05..=0x06 => Box::new(MBC2::new()),
            0x0F..=0x13 => Box::new(MBC3::new()),
            0x19..=0x1E => Box::new(MBC5::new()),
            _ => {
                println!("This ROM is type {:#04X} but it's not implemented yet. Expect bad things XD", ctr_type);
                Box::new(NoMBC)
            } 
        };

        let has_battery = match ctr_type {
           0x03 => true,
           0x06 => true,
           0x0F | 0x10 | 0x13 => true,
           0x1E | 0x1B => true,
           _ => false
        };

        let ram = match ctr_type {
            0x05 | 0x06 => {
                Some(vec![0; 0x200])
            },

            _ => {
                match rom.get(RAM_TYPE_ADDR as usize).copied().unwrap() {
                    ERAM_NONE => None,
                    ERAM_8KB => Some(vec![0; 0x2000]),
                    ERAM_32KB => Some(vec![0; 0x8000]),
                    ERAM_64KB => Some(vec![0; 0x10000]),
                    ERAM_128KB => Some(vec![0; 0x20000]),
                    weirdo => {
                        print!("Extra RAM type {:#04X} not recognized. Expect bad things XD", weirdo);
                        None
                    }
                }
            }
        };
        
        let bytes: &[u8] = &rom[TITLE_BEGIN as usize ..= TITLE_END as usize];
        let text = String::from_utf8_lossy(bytes);
        
        Ok(Cartridge { 
            title: text.trim_end_matches("\0").to_string(), 
            rom,
            ram,
            ram_written: false,
            mbc,
            has_battery
        })
    }

    pub fn has_battery(&self) -> bool {
        self.has_battery
    }

    pub fn get_ram(&self) -> Option<Vec<u8>> {
        match &self.ram {
            None => None,
            Some(ram) => {
                let mut extended = ram.clone();
                extended.extend(self.mbc.rtc_save());
                Some(extended)
            }
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
                let ram_len = ram.len();
                if lram.len() >= ram_len {
                    ram.copy_from_slice(&lram[0..ram_len]);
                    self.mbc.rtc_load(&lram[ram_len..]);
                    Ok(())
                }else{
                    Err("SAV data is corrupted or not compatible with this game".to_string())
                }
            }
        }
    }

    pub fn tick_mbc(&mut self, t: u8) {
        self.mbc.tick(t);
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            ..=BANK1_END => {
                let raddr = self.mbc.rom_offset(addr);
                self.rom[raddr % self.rom.len()]
            },
            ERAM_BEGIN..=ERAM_END => {
                match self.mbc.rtc_read(){
                    Some(value) => value,
                    None => {
                        let raddr = self.mbc.ram_offset(addr);
                        match raddr {
                            None => 0xFF,
                            Some(a) => {
                                match &self.ram {
                                    None => 0xFF,
                                    Some(ram) => self.mbc.ram_read(ram[a % ram.len()])
                                }
                            }
                        }
                    }
                }
            },
            _ => 0xFF
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            ..=0x7FFF => self.mbc.write_register(addr, value),
            ERAM_BEGIN..=ERAM_END => {
                if !self.mbc.rtc_write(value){
                    let raddr = self.mbc.ram_offset(addr);
                    match raddr {
                        None => return,
                        Some(a) => {
                            match &mut self.ram {
                                None => return,
                                Some(ram) => { 
                                    let ram_len = ram.len();
                                    ram[a % ram_len] = value;
                                    self.ram_written = true;
                                }
                            }
                        }
                    }
                }
            }
            _ => ()
        }
    }

}

pub trait MBC: std::fmt::Debug {
    fn mbc_info(&self) -> String;

    fn tick(&mut self, _t: u8) {}

    fn rom_offset(&self, addr: u16) -> usize;

    fn ram_offset(&self, addr: u16) -> Option<usize>;

    fn ram_read(&self, value: u8) -> u8 { value }

    fn rtc_read(&self) -> Option<u8> { None }

    fn rtc_write(&mut self, _value: u8) -> bool { false }

    fn rtc_save(&self) -> Vec<u8> { Vec::new() }

    fn rtc_load(&mut self, _data: &[u8]) {} 

    fn write_register(&mut self, addr: u16, value: u8); 
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

        // ---------- MBC5 ----------

    #[test]
    fn mbc5_rom_bank_switching() {
        let mut cart = make_rom(0x19, 0x00, 8);

        assert_eq!(cart.read(0x4000), 1);

        cart.write(0x2000, 5);
        assert_eq!(cart.read(0x4000), 5);

        // Unlike MBC1, bank 0 can be selected
        cart.write(0x2000, 0);
        assert_eq!(cart.read(0x4000), 0);
    }

    #[test]
    fn mbc5_rom_bank_bit8() {
        let mut cart = make_rom(0x19, 0x00, 512); // 8 MB
        // Mark bank 0x101 so it can be told apart from bank 0x01
        cart.rom[0x101 * 0x4000 + 1] = 0xAB;

        cart.write(0x2000, 0x01);
        assert_eq!(cart.read(0x4001), 0x00); // bank 0x001

        cart.write(0x3000, 0x01);
        assert_eq!(cart.read(0x4001), 0xAB); // bank 0x101

        // Changing the low byte keeps bit 8
        cart.write(0x2000, 0x02);
        assert_eq!(cart.read(0x4000), 0x02); // bank 0x102 (tag is the low byte)
        cart.write(0x3000, 0x00);
        cart.write(0x2000, 0x01);
        assert_eq!(cart.read(0x4001), 0x00); // back to bank 0x001
    }

    #[test]
    fn mbc5_ram_banks() {
        let mut cart = make_rom(0x1B, 0x04, 4); // MBC5 + RAM + battery, 128 KB of RAM

        cart.write(0x0000, 0x0A);

        // Bank 0
        cart.write(0xA000, 0x11);
        // Bank 15
        cart.write(0x4000, 0x0F);
        cart.write(0xA000, 0xFF);
        assert_eq!(cart.read(0xA000), 0xFF);

        // Back to bank 0
        cart.write(0x4000, 0x00);
        assert_eq!(cart.read(0xA000), 0x11);

        // RAM layout must match the real chip (so .sav files are compatible)
        let ram = cart.get_ram().unwrap();
        assert_eq!(ram[0], 0x11);
        assert_eq!(ram[15 * 0x2000], 0xFF);

        // Disable
        cart.write(0x0000, 0x00);
        assert_eq!(cart.read(0xA000), 0xFF);
    }

        // ---------- MBC2 ----------

    #[test]
    fn mbc2_rom_bank_switching() {
        let mut cart = make_rom(0x05, 0x00, 16); // 256 KB

        assert_eq!(cart.read(0x4000), 1);

        // Address bit 8 = 1 -> ROM bank register
        cart.write(0x2100, 5);
        assert_eq!(cart.read(0x4000), 5);

        // Only 4 bits are used
        cart.write(0x0100, 0x1F);
        assert_eq!(cart.read(0x4000), 0x0F);

        // Bank 0 -> 1
        cart.write(0x2100, 0);
        assert_eq!(cart.read(0x4000), 1);

        // Address bit 8 = 0 -> does NOT change the ROM bank
        cart.write(0x2000, 5);
        assert_eq!(cart.read(0x4000), 1);
    }

    #[test]
    fn mbc2_ram() {
        let mut cart = make_rom(0x06, 0x00, 2); // MBC2 + battery, header says no RAM

        // Disabled: 0xFF
        assert_eq!(cart.read(0xA000), 0xFF);

        // Enable (address bit 8 = 0)
        cart.write(0x0000, 0x0A);

        // Only the low 4 bits are stored, the high 4 read as 1
        cart.write(0xA000, 0x35);
        assert_eq!(cart.read(0xA000), 0xF5);

        // 512 positions, mirrored over 0xA000-0xBFFF
        cart.write(0xA1FF, 0x07);
        assert_eq!(cart.read(0xA1FF), 0xF7);
        assert_eq!(cart.read(0xA200), 0xF5); // mirror of 0xA000
        assert_eq!(cart.read(0xBFFF), 0xF7); // mirror of 0xA1FF

        // Disable again
        cart.write(0x0000, 0x00);
        assert_eq!(cart.read(0xA000), 0xFF);

        // Battery
        assert!(cart.has_battery());
    }

        // ---------- MBC3 RTC ----------

    fn tick_seconds(cart: &mut Cartridge, secs: u32) {
        for _ in 0..secs * (4_194_304 / 4) { cart.tick_mbc(4); }
    }

    // Latch, then read the 5 RTC registers [S, M, H, DL, DH]
    fn read_rtc(cart: &mut Cartridge) -> [u8; 5] {
        cart.write(0x6000, 0x00);
        cart.write(0x6000, 0x01);
        let mut regs = [0; 5];
        for i in 0..5 {
            cart.write(0x4000, 0x08 + i as u8);
            regs[i] = cart.read(0xA000);
        }
        regs
    }

    fn write_rtc(cart: &mut Cartridge, regs: [u8; 5]) {
        for i in 0..5 {
            cart.write(0x4000, 0x08 + i as u8);
            cart.write(0xA000, regs[i]);
        }
    }

    #[test]
    fn mbc3_rtc_counts_seconds() {
        let mut cart = make_rom(0x10, 0x03, 4);
        cart.write(0x0000, 0x0A);

        tick_seconds(&mut cart, 2);
        assert_eq!(read_rtc(&mut cart), [2, 0, 0, 0, 0]);
    }

    #[test]
    fn mbc3_rtc_carries() {
        let mut cart = make_rom(0x10, 0x03, 4);
        cart.write(0x0000, 0x0A);

        // Day 255, 23:59:59 -> day 256 (bit 8 goes to DH bit 0)
        write_rtc(&mut cart, [59, 59, 23, 0xFF, 0x00]);
        tick_seconds(&mut cart, 1);
        assert_eq!(read_rtc(&mut cart), [0, 0, 0, 0x00, 0x01]);

        // Day 511, 23:59:59 -> day 0 + overflow flag (bit 7)
        write_rtc(&mut cart, [59, 59, 23, 0xFF, 0x01]);
        tick_seconds(&mut cart, 1);
        assert_eq!(read_rtc(&mut cart), [0, 0, 0, 0x00, 0x80]);
    }

    #[test]
    fn mbc3_rtc_latch_is_a_snapshot() {
        let mut cart = make_rom(0x10, 0x03, 4);
        cart.write(0x0000, 0x0A);

        assert_eq!(read_rtc(&mut cart)[0], 0); // latched at 0 s
        tick_seconds(&mut cart, 1);

        // Without a new latch, still the old snapshot
        cart.write(0x4000, 0x08);
        assert_eq!(cart.read(0xA000), 0);

        // A second latch must work too
        assert_eq!(read_rtc(&mut cart)[0], 1);
    }

    #[test]
    fn mbc3_rtc_halt() {
        let mut cart = make_rom(0x10, 0x03, 4);
        cart.write(0x0000, 0x0A);

        write_rtc(&mut cart, [10, 0, 0, 0, 0x40]); // halted
        tick_seconds(&mut cart, 2);
        assert_eq!(read_rtc(&mut cart), [10, 0, 0, 0, 0x40]);
    }

    #[test]
    fn mbc3_rtc_and_ram_do_not_mix() {
        let mut cart = make_rom(0x10, 0x03, 4);
        cart.write(0x0000, 0x0A);

        cart.write(0x4000, 0x00); // RAM bank 0
        cart.write(0xA000, 0x42);

        cart.write(0x4000, 0x08); // RTC seconds
        cart.write(0xA000, 0x05);

        cart.write(0x4000, 0x00); // back to RAM
        assert_eq!(cart.read(0xA000), 0x42);
    }

        #[test]
    fn mbc3_rtc_save_roundtrip() {
        let mut cart = make_rom(0x10, 0x03, 4);
        cart.write(0x0000, 0x0A);
        cart.write(0x4000, 0x00);
        cart.write(0xA000, 0x42);
        write_rtc(&mut cart, [10, 20, 5, 0x34, 0x40]); // halted: loading must not advance it

        let sav = cart.get_ram().unwrap();
        assert_eq!(sav.len(), 0x8000 + 48); // 32 KB RAM + 48 bytes RTC

        let mut cart2 = make_rom(0x10, 0x03, 4);
        cart2.load_ram(&sav).unwrap();
        cart2.write(0x0000, 0x0A);
        assert_eq!(read_rtc(&mut cart2), [10, 20, 5, 0x34, 0x40]);
        cart2.write(0x4000, 0x00);
        assert_eq!(cart2.read(0xA000), 0x42);
    }

    #[test]
    fn mbc3_rtc_load_adds_elapsed_time() {
        let mut cart = make_rom(0x10, 0x03, 4);

        // RAM + clock at 00:00:00 day 0 (running) + latch, saved one hour ago
        let mut sav = vec![0u8; 0x8000 + 40];
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        sav.extend_from_slice(&(now - 3600).to_le_bytes());

        cart.load_ram(&sav).unwrap();
        cart.write(0x0000, 0x0A);
        let regs = read_rtc(&mut cart);
        assert_eq!((regs[1], regs[2]), (0, 1)); // 0 minutes, 1 hour
    }

    #[test]
    fn mbc3_loads_sav_of_any_known_size() {
        let mut cart = make_rom(0x10, 0x03, 4);

        // Old .sav, RAM only
        assert!(cart.load_ram(&vec![0u8; 0x8000]).is_ok());

        // 44-byte format (timestamp as u32), clock halted so it doesn't advance
        let mut sav = vec![0u8; 0x8000 + 44];
        sav[0x8000 + 16] = 0x40; // DH = halt
        assert!(cart.load_ram(&sav).is_ok());

        // Shorter than RAM -> error
        assert!(cart.load_ram(&vec![0u8; 0x100]).is_err());
    }

}