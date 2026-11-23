pub const PPU_IF_BIT: u8 = 0;

const LINE_CYCLES: u16 = 456;
const VBLANK_LINE: u8 = 144;
const TOTAL_LINES: u8 = 154;


pub struct PPU {
    line_progress: u16,
    ly: u8
}

impl PPU {
    
    pub fn new() -> PPU {
        PPU {
            line_progress: 0, // 1 line -> 456 t-cycles
            ly: 0             // 1 draw -> 153 (vblank 144)
        }
    }

    pub fn ly(&self) -> u8 {
        self.ly
    }

    fn update_line(&mut self) -> bool {
        let mut vblank = false;
        self.line_progress += 1;
        if self.line_progress as u16 == LINE_CYCLES {
            self.line_progress = 0;
            self.ly += 1;
            if self.ly == TOTAL_LINES {
                self.ly = 0;
            }
            vblank = self.ly == VBLANK_LINE;
        }
        vblank
    }

    pub fn tick(&mut self, t: u8) -> bool {
        let mut vblank: bool = false;
        for _ in 0..t {
            vblank |= self.update_line();
        }
        vblank
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
                let vblank = ppu.tick(1);
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
}