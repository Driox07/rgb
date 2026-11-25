use std::collections::VecDeque;

use super::Cartridge;
use super::Timer;
use super::PPU;


const ROM_END:    u16 = 0x7FFF;
const VRAM_BEGIN: u16 = 0x8000;
const VRAM_END:   u16 = 0x9FFF;
const ERAM_BEGIN: u16 = 0xA000;
const ERAM_END:   u16 = 0xBFFF;
const WRAM_BEGIN: u16 = 0xC000;
const WRAM_END:   u16 = 0xDFFF;
const ECHO_BEGIN: u16 = 0xE000;
const ECHO_END:   u16 = 0xFDFF;
const OAM_BEGIN:  u16 = 0xFE00;
const OAM_END:    u16 = 0xFE9F;
const NU_BEGIN:   u16 = 0xFEA0;
const NU_END:     u16 = 0xFEFF;
const IO_BEGIN:   u16 = 0xFF00;
const IO_END:     u16 = 0xFF7F;
const HRAM_BEGIN: u16 = 0xFF80;
const HRAM_END:   u16 = 0xFFFE;
const IE_ADDR:    u16 = 0xFFFF;

pub struct Bus {
    rom:  Option<Cartridge>,
    wram: [u8; 0x2000],
    io:   [u8; 0x0080],
    hram: [u8; 0x007F],
    ie:    u8,
    timer: Timer,
    ppu: PPU,
    dma: u8,
    serial: VecDeque<u8>
}

impl Bus{

    pub fn new() -> Bus {
        Bus{
            rom:  None,
            wram: [0; 0x2000],
            io:   [0; 0x0080],
            hram: [0; 0x007F],
            ie:    0,
            timer: Timer::new(),
            ppu: PPU::new(),
            dma: 0,
            serial: VecDeque::new()
        }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) -> Result<(), String> {
        match Cartridge::new(rom) {
            Ok(cartridge) => {
                self.rom = Some(cartridge);
                Ok(())
            },
            Err(error) => Err(format!("Couldn't load rom: {}", error))
        }
    }

    pub fn tick_timer(&mut self, t: u8) {
        if self.timer.tick(t) {
            self.request_interrupt(super::timer::TIMER_IF_BIT);
        }
    }

    pub fn tick_ppu(&mut self, t: u8) -> bool{
        let int_mask = self.ppu.tick(t);
        self.write(0xFF0F, self.read(0xFF0F) | int_mask);
        int_mask & 1 == 1
    }

    pub fn ppu_framebuffer(&mut self) -> &[u8] {
        self.ppu.frame_buffer()
    }

    pub fn fetch_serial(&mut self) -> Option<u8> {
        self.serial.pop_front()
    }

    pub fn interrupt_pending(&self) -> u8 {
        self.ie & self.read(0xFF0F) & 0x1F
    }

    pub fn request_interrupt(&mut self, i: u8) {
        self.write(0xFF0F, self.read(0xFF0F) | (1 << i));
    }

    pub fn clear_interrupt(&mut self, i: u8) {
        self.write(0xFF0F, self.read(0xFF0F) & !(1 << i));
    } 

    fn dma_copy(&mut self) {
        let addr = (self.dma as u16) << 8;
        for i in 0..0xA0 {
            self.write(OAM_BEGIN + i, self.read(addr + i));
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            ..=ROM_END   => match &self.rom{
                Some(c) => c.read(addr),
                None => 0xFF
            },
            VRAM_BEGIN..=VRAM_END => self.ppu.read_vram(addr - VRAM_BEGIN),
            ERAM_BEGIN..=ERAM_END => match &self.rom{
                Some(c) => c.read(addr),
                None => 0xFF
            },
            WRAM_BEGIN..=WRAM_END => self.wram[(addr - WRAM_BEGIN) as usize],
            ECHO_BEGIN..=ECHO_END => self.wram[(addr - ECHO_BEGIN) as usize],
            OAM_BEGIN..=OAM_END   => self.ppu.read_oam(addr - OAM_BEGIN),
            NU_BEGIN..=NU_END     => 0xFF,
            IO_BEGIN..=IO_END     => {

                match addr {
                    
                    0xFF04 => self.timer.div(),

                    0xFF05 => self.timer.tima(),

                    0xFF06 => self.timer.tma(),

                    0xFF07 => self.timer.tac(),

                    0xFF0F => {
                        self.io[0x0F] | 0xE0
                    },

                    0xFF40 => self.ppu.lcdc(),

                    0xFF41 => self.ppu.stat(),

                    0xFF42 => self.ppu.scy(),

                    0xFF43 => self.ppu.scx(),

                    0xFF44 => {
                        self.ppu.ly()
                    },

                    0xFF45 => self.ppu.lyc(),

                    0xFF46 => self.dma,

                    0xFF47 => self.ppu.bgp(),

                    0xFF48 => self.ppu.obp0(),

                    0xFF49 => self.ppu.obp1(),

                    _=> self.io  [(addr - IO_BEGIN)   as usize]
                }
            },
            HRAM_BEGIN..=HRAM_END => self.hram[(addr - HRAM_BEGIN) as usize],
            IE_ADDR..            => self.ie
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            VRAM_BEGIN..=VRAM_END => self.ppu.write_vram(addr - VRAM_BEGIN, value),
            ERAM_BEGIN..=ERAM_END => match &mut self.rom{
                Some(c) => c.write(addr, value),
                None => ()
            },
            WRAM_BEGIN..=WRAM_END => self.wram[(addr - WRAM_BEGIN) as usize] = value,
            ECHO_BEGIN..=ECHO_END => self.wram[(addr - ECHO_BEGIN) as usize] = value,
            OAM_BEGIN..=OAM_END   => self.ppu.write_oam(addr - OAM_BEGIN, value),
            IO_BEGIN..=IO_END     => {
                
                match addr {
                    0xFF02 => {
                        if value == 0x81 { self.serial.push_back(self.io[1]); }
                        self.io [(addr - IO_BEGIN) as usize] = value
                    },

                    0xFF04 => self.timer.set_div(),

                    0xFF05 => self.timer.set_tima(value),

                    0xFF06 => self.timer.set_tma(value),

                    0xFF07 => self.timer.set_tac(value),

                    0xFF40 => self.ppu.set_lcdc(value),
                    
                    0xFF41 => self.ppu.set_stat(value),

                    0xFF42 => self.ppu.set_scy(value),

                    0xFF43 => self.ppu.set_scx(value),

                    0xFF45 => self.ppu.set_lyc(value),

                    0xFF46 => {
                        self.dma = value;
                        self.dma_copy();
                    },

                    0xFF47 => self.ppu.set_bgp(value),

                    0xFF48 =>  self.ppu.set_obp0(value),

                    0xFF49 =>  self.ppu.set_obp1(value),

                    _ => self.io [(addr - IO_BEGIN) as usize] = value,
                }
            },
            HRAM_BEGIN..=HRAM_END => self.hram[(addr - HRAM_BEGIN) as usize] = value,
            IE_ADDR..             => self.ie = value,
            _ => ()
        }
    }

    pub fn read16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr.wrapping_add(1))])
    }

    pub fn write16(&mut self, addr: u16, value: u16) -> () {
        let bytes = u16::to_le_bytes(value);
        self.write(addr, bytes[0]);
        self.write(addr.wrapping_add(1), bytes[1]);
    }

}

#[cfg(test)]
mod tests {
use std::assert_matches;

use super::*;

    #[test]
    fn write_read_rom(){
        let mut bus = Bus::new();
        assert_eq!(bus.read(0x000), 0xFF);
        assert_eq!(bus.read(0x4000), 0xFF);
        assert_eq!(bus.read(0x4077), 0xFF);
        assert_eq!(bus.read(0x40EF), 0xFF);
        let rom = std::fs::read("./test_roms/valid_rom.gb");
        assert_matches!(rom, Ok(_));
        assert_matches!(bus.load_rom(rom.unwrap()), Ok(_));
        assert_eq!(bus.read(0x4000), 0xC3);
        assert_eq!(bus.read(0x4077), 0xE0);
        assert_eq!(bus.read(0x40EF), 0x00);
        bus.write(0x4000, 0xFF);
        assert_eq!(bus.read(0x4000), 0xC3);
    }

    #[test]
    fn write_read_vram(){
        let mut bus = Bus::new();
        bus.write(VRAM_BEGIN, 0x84);
        assert_eq!(bus.read(VRAM_BEGIN), 0x84);
        bus.write(VRAM_BEGIN + 20, 0x29);
        assert_eq!(bus.read(VRAM_BEGIN + 20), 0x29);
        bus.write(VRAM_END, 0x24);
        assert_eq!(bus.read(VRAM_END), 0x24);
    }

    #[test]
    fn write_read_wram(){
        let mut bus = Bus::new();
        bus.write(WRAM_BEGIN, 0x84);
        assert_eq!(bus.read(WRAM_BEGIN), 0x84);
        bus.write(WRAM_BEGIN + 20, 0x29);
        assert_eq!(bus.read(WRAM_BEGIN + 20), 0x29);
        bus.write(WRAM_END, 0x24);
        assert_eq!(bus.read(WRAM_END), 0x24);
    }

    #[test]
    fn write_read_echo(){
        let mut bus = Bus::new();
        bus.write(WRAM_BEGIN, 0x84);
        assert_eq!(bus.read(ECHO_BEGIN), 0x84);
        bus.write(ECHO_BEGIN + 20, 0x29);
        assert_eq!(bus.read(WRAM_BEGIN + 20), 0x29);
        bus.write(ECHO_END, 0x24);
        assert_eq!(bus.read(WRAM_END), 0x00);
        assert_eq!(bus.read(ECHO_END), 0x24);
    }

    #[test]
    fn write_read_oam(){
        let mut bus = Bus::new();
        bus.write(OAM_BEGIN, 0x84);
        assert_eq!(bus.read(OAM_BEGIN), 0x84);
        bus.write(OAM_BEGIN + 20, 0x29);
        assert_eq!(bus.read(OAM_BEGIN + 20), 0x29);
        bus.write(OAM_END, 0x24);
        assert_eq!(bus.read(OAM_END), 0x24);
    }

    #[test]
    fn write_read_nu(){
        let mut bus = Bus::new();
        bus.write(NU_BEGIN, 0x84);
        assert_eq!(bus.read(NU_BEGIN), 0xFF);
        bus.write(NU_BEGIN + 20, 0x29);
        assert_eq!(bus.read(NU_BEGIN + 20), 0xFF);
        bus.write(NU_END, 0x24);
        assert_eq!(bus.read(NU_END), 0xFF);
    }

    #[test]
    fn write_read_io(){
        let mut bus = Bus::new();
        bus.write(IO_BEGIN, 0x84);
        assert_eq!(bus.read(IO_BEGIN), 0x84);
        bus.write(IO_BEGIN + 20, 0x29);
        assert_eq!(bus.read(IO_BEGIN + 20), 0x29);
        bus.write(IO_END, 0x24);
        assert_eq!(bus.read(IO_END), 0x24);
    }

    #[test]
    fn write_read_hram(){
        let mut bus = Bus::new();
        bus.write(HRAM_BEGIN, 0x84);
        assert_eq!(bus.read(HRAM_BEGIN), 0x84);
        bus.write(HRAM_BEGIN + 20, 0x29);
        assert_eq!(bus.read(HRAM_BEGIN + 20), 0x29);
        bus.write(HRAM_END, 0x24);
        assert_eq!(bus.read(HRAM_END), 0x24);
    }

    #[test]
    fn write_read_ie(){
        let mut bus = Bus::new();
        bus.write(IE_ADDR, 0x95);
        assert_eq!(bus.read(IE_ADDR), 0x95);
    }

    #[test]
    fn write_read_16(){
        let mut bus = Bus::new();
        bus.write16(WRAM_BEGIN, 0x3284);
        assert_eq!(bus.read16(WRAM_BEGIN), 0x3284);
        assert_eq!(bus.read(WRAM_BEGIN), 0x84);
        assert_eq!(bus.read(WRAM_BEGIN+1), 0x32);

        bus.write16(OAM_BEGIN + 25, 0x1234);
        assert_eq!(bus.read16(OAM_BEGIN + 25), 0x1234);
        assert_eq!(bus.read(OAM_BEGIN + 25), 0x34);
        assert_eq!(bus.read(OAM_BEGIN + 26), 0x12);

        bus.write16(WRAM_END, 0xAABB);
        assert_eq!(bus.read16(WRAM_END), 0xAABB);
        assert_eq!(bus.read(WRAM_END), 0xBB);
        assert_eq!(bus.read(WRAM_BEGIN), 0xAA);
    }

    #[test]
    fn test_dma_copies_160_bytes_to_oam() {
        let mut bus = Bus::new();
        for i in 0..0xA0u16 {
            bus.write(0xC000 + i, i as u8);
        }
        bus.write(0xFF46, 0xC0); // source = 0xC000
        for i in 0..0xA0u16 {
            assert_eq!(bus.read(0xFE00 + i), i as u8);
        }
    }
 
}