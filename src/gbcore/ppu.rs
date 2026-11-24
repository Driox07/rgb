pub const PPU_IF_BIT: u8 = 0;

pub const PPU_MODE_HBLANK: u8 = 0;
pub const PPU_MODE_VBLANK: u8 = 1;
pub const PPU_MODE_OAM: u8 = 2;
pub const PPU_MODE_DRAWING: u8 = 3;

const LINE_CYCLES: u16 = 456;
const VBLANK_LINE: u8 = 144;
const TOTAL_LINES: u8 = 154;


pub struct PPU {
    line_progress: u16,
    ly: u8,
    stat: u8,
    lyc: u8,
    last_stat_state: bool,

    lcdc: u8,
    scy: u8,
    scx: u8,
    bgp: u8
}

impl PPU {
    
    pub fn new() -> PPU {
        PPU {
            line_progress: 0, // 1 line -> 456 t-cycles
            ly: 0,            // 1 draw -> 153 (vblank 144)
            stat: 0x80,
            lyc: 0,
            last_stat_state: false,
            lcdc: 0,
            scy: 0,
            scx: 0,
            bgp: 0
        }
    }

    fn mode(&self) -> u8 {
        let mut mode = PPU_MODE_VBLANK;
        if self.ly < 144 {
            mode = match self.line_progress {
                0..=79 => PPU_MODE_OAM,
                80..=251 => PPU_MODE_DRAWING,
                252..=455 => PPU_MODE_HBLANK,
                _ => PPU_MODE_VBLANK
            }
        }
        mode
    }

    pub fn lcdc(&self) -> u8 {
        self.lcdc
    }

    pub fn set_lcdc(&mut self, value: u8) {
        self.lcdc = value;
    }

    pub fn scy(&self) -> u8 {
        self.scy
    }

    pub fn set_scy(&mut self, value: u8) {
        self.scy = value;
    }

    pub fn scx(&self) -> u8 {
        self.scx
    }

    pub fn set_scx(&mut self, value: u8) {
        self.scx = value;
    }

    pub fn bgp(&self) -> u8 {
        self.bgp
    }

    pub fn set_bgp(&mut self, value: u8) {
        self.bgp = value;
    }

    pub fn lyc(&self) -> u8 {
        self.lyc
    }

    pub fn set_lyc(&mut self, value: u8) {
        self.lyc = value;
    }

    pub fn stat(&self) -> u8 {
        self.stat
    }

    pub fn set_stat(&mut self, value: u8) {
        self.stat &= !0x78;
        self.stat |= value & 0x78;
    }

    pub fn ly(&self) -> u8 {
        self.ly
    }

    fn update_line(&mut self) -> u8 {
        let mut vblank = 0;
        self.line_progress += 1;
        if self.line_progress as u16 == LINE_CYCLES {
            self.line_progress = 0;
            self.ly += 1;
            if self.ly == TOTAL_LINES {
                self.ly = 0;
            }
            if self.ly == VBLANK_LINE{
                vblank = 1;
            }
        }
        vblank
    }

    fn update_stat(&mut self) -> u8 {
        self.stat |= 0x80;

        if self.ly == self.lyc() {
            self.stat |= 0x04;
        }else{
            self.stat &= !0x04;
        }

        self.stat &= !0x03;
        self.stat |= self.mode();

        self.check_stat_int()
    }

    fn check_stat_int(&mut self) -> u8 {
        let mut int_mask: u8 = 0;
        // bit 6 of stat and ly == lyc
        let stat_int =  ((self.stat & 0x40) != 0) && (self.stat & 0x04 != 0)
                           || ((self.stat & 0x20) != 0) && (self.mode() == PPU_MODE_OAM)
                           || ((self.stat & 0x10) != 0) && (self.mode() == PPU_MODE_VBLANK)
                           || ((self.stat & 0x08) != 0) && (self.mode() == PPU_MODE_HBLANK);

        if !self.last_stat_state && stat_int {
            int_mask = 2;
        }
        self.last_stat_state = stat_int;
        int_mask
    }

    pub fn tick(&mut self, t: u8) -> u8 {
        let mut int_mask: u8 = 0;
        for _ in 0..t {
            int_mask |= self.update_line();
            int_mask |= self.update_stat();
        }
        int_mask
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Bus;

    #[test]
    fn proper_vblank() {
        let mut ppu = PPU::new();
        let total_ticks = LINE_CYCLES as u32 * TOTAL_LINES as u32;
        let vblank_tick = LINE_CYCLES as u32 * VBLANK_LINE as u32;
        for _ in 0..2{
            for i in 0..total_ticks {
                let vblank = ppu.tick(1) & 1 != 0;
                if i == vblank_tick - 1 {
                    assert!(vblank);
                }else{
                    assert!(!vblank);
                }
            }
            assert_eq!(ppu.ly, 0);
        }
        let mut bus = Bus::new();
        bus.write(0xFFFF, 0x01);
        assert!(bus.interrupt_pending() == 0);
        for i in 0..total_ticks {
            bus.tick_ppu(1);
            if i == vblank_tick {
                assert!(bus.interrupt_pending() == 1);
            }
        }
    }

    fn advance_to_line(ppu: &mut PPU, target_ly: u8) {
        while ppu.ly() != target_ly {
            ppu.tick(1);
        }
    }

    #[test]
    fn test_ppu_mode_transitions_on_ly10_and_vblank() {
        let mut ppu = PPU::new();

        // Advance to dot 0 of LY = 10
        advance_to_line(&mut ppu, 10);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_OAM);

        // Dot 79 -> Mode 2 (OAM)
        ppu.tick(79);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_OAM);

        // Dot 80 -> Mode 3 (Drawing)
        ppu.tick(1);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_DRAWING);

        // Dot 251 -> Mode 3 (Drawing)
        ppu.tick(171);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_DRAWING);

        // Dot 252 -> Mode 0 (HBlank)
        ppu.tick(1);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_HBLANK);

        // Dot 455 -> Mode 0 (HBlank)
        ppu.tick(203);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_HBLANK);

        // LY = 144 -> Mode 1 (VBlank)
        advance_to_line(&mut ppu, 144);
        assert_eq!(ppu.stat() & 0x03, PPU_MODE_VBLANK);
    }

    #[test]
    fn test_lyc_coincidence_flag_bit2() {
        let mut ppu = PPU::new();
        ppu.set_lyc(5);

        // End of LY = 4 (dot 455 = 255 + 200) -> bit 2 must be 0
        advance_to_line(&mut ppu, 4);
        ppu.tick(255);
        ppu.tick(200);
        assert_eq!(ppu.stat() & 0x04, 0);

        // Enter LY = 5 (dot 0) -> bit 2 must be 1
        ppu.tick(1);
        assert_eq!(ppu.ly(), 5);
        assert_ne!(ppu.stat() & 0x04, 0);

        // End of LY = 5 (dot 455 = 255 + 200) -> bit 2 remains 1
        ppu.tick(255);
        ppu.tick(200);
        assert_ne!(ppu.stat() & 0x04, 0);

        // Enter LY = 6 (dot 0) -> bit 2 must clear to 0
        ppu.tick(1);
        assert_eq!(ppu.ly(), 6);
        assert_eq!(ppu.stat() & 0x04, 0);
    }

    #[test]
    fn test_lyc_stat_interrupt_fires_once_at_line_5() {
        let mut ppu = PPU::new();
        ppu.set_lyc(5);
        // Enable LYC=LY STAT interrupt (bit 6)
        ppu.set_stat(0x40);

        let mut stat_irqs = 0;

        for line in 0..7 {
            for _ in 0..456 {
                let int_mask = ppu.tick(1);
                if (int_mask & 0x02) != 0 {
                    stat_irqs += 1;
                    assert_eq!(ppu.ly(), 5);
                }
                let _ = line;
            }
        }

        assert_eq!(stat_irqs, 1);
    }

    #[test]
    fn test_stat_blocking_rising_edge_only() {
        let mut ppu = PPU::new();
        // Enable Mode 0 (bit 3 = 0x08) and Mode 1 (bit 4 = 0x10) STAT interrupts
        ppu.set_stat(0x18);

        // Advance to LY = 143, dot 252 -> enters Mode 0 (STAT line rises 0 -> 1)
        advance_to_line(&mut ppu, 143);
        ppu.tick(251);
        let int_mode0 = ppu.tick(1);
        assert_ne!(int_mode0 & 0x02, 0);

        // Remaining dots of Mode 0 on LY = 143 (dots 253..=454) -> no new STAT IRQ
        for _ in 253..=455 {
            let int_mask = ppu.tick(1);
            assert_eq!(int_mask & 0x02, 0);
        }

        // Dot 455 -> transitions directly from Mode 0 (LY=143) to Mode 1 (LY=144).
        // VBlank IRQ (bit 0) fires, STAT IRQ (bit 1) is blocked because STAT line stays high.
        let int_vblank = ppu.tick(1);
        assert_ne!(int_vblank & 0x01, 0);
        assert_eq!(int_vblank & 0x02, 0);
    }

    #[test]
    fn test_stat_write_preserves_readonly_bits_and_sets_bit7() {
        let mut ppu = PPU::new();
        ppu.set_lyc(0);
        ppu.tick(1); // Dot 1 on LY=0 -> Mode 2 (0x02) and LY==LYC (0x04) -> lower 3 bits = 0x06

        ppu.set_stat(0xFF);
        let stat = ppu.stat();

        assert_ne!(stat & 0x80, 0);
        assert_eq!(stat & 0x78, 0x78);
        assert_eq!(stat & 0x07, 0x06);
    }

}