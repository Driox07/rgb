pub const PPU_IF_BIT: u8 = 0;

pub const PPU_MODE_HBLANK: u8 = 0;
pub const PPU_MODE_VBLANK: u8 = 1;
pub const PPU_MODE_OAM: u8 = 2;
pub const PPU_MODE_DRAWING: u8 = 3;

pub const LCDC_LCDPPU_ENABLE: u8 = 0x80;
pub const LCDC_WINDOW_TILE_MAP: u8 = 0x40;
pub const LCDC_WINDOW_ENABLE: u8 = 0x20;
pub const LCDC_BG_WINDOW_TILES: u8 = 0x10;
pub const LCDC_BG_TILE_MAP: u8 = 0x08;
pub const LCDC_OBJ_SIZE: u8 = 0x04;
pub const LCDC_OBJ_ENABLE: u8 = 0x02;
pub const LCDC_BG_WINDOW_ENABLE: u8 = 0x01;

const LINE_CYCLES: u16 = 456;
const HBLANK_CYCLE: u16 = 252;
const VBLANK_LINE: u8 = 144;
const TOTAL_LINES: u8 = 154;

const VRAM_BASE: u16 = 0x8000;

const TILE_AREA_0: u16 = 0x9000;
const TILE_AREA_1: u16 = 0x8000;

const TILE_MAP_A: u16 = 0x9800;
const TILE_MAP_B: u16 = 0x9C00;

const TILE_DATA_BYTES: u16 = 16;
const TILE_MAP_SIZE: u16 = 32;

const SCREEN_W: u8 = 160;
const SCREEN_H: u8 = 144;

pub struct PPU {
    vram: [u8; 0x2000],
    frame_buffer: [u8; (SCREEN_W as u16 * SCREEN_H as u16) as usize],
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
            vram: [0; 0x2000],
            frame_buffer: [0; 160 * 144],
            line_progress: 0,
            ly: 0,
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
        if self.ly < VBLANK_LINE {
            mode = match self.line_progress {
                0..=79 => PPU_MODE_OAM,
                80..=251 => PPU_MODE_DRAWING,
                252..=455 => PPU_MODE_HBLANK,
                _ => PPU_MODE_VBLANK
            }
        }
        mode
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[addr as usize]
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[addr as usize] = value;
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

    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    fn update_line(&mut self) -> u8 {
        let mut vblank = 0;
        self.line_progress += 1;
        if self.ly < VBLANK_LINE && self.line_progress == HBLANK_CYCLE {
            self.draw_line();
        }
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

    pub fn get_tile_color(&self, tile_index: u8, tile_row: u8, tile_col: u8) -> u8 {
        let offset = if self.lcdc & LCDC_BG_WINDOW_TILES == 0{
            ((TILE_AREA_0 as i16 + (tile_index as i8) as i16 * TILE_DATA_BYTES as i16)) as u16 - VRAM_BASE
        }else{
            (TILE_AREA_1 + (tile_index as u16) * TILE_DATA_BYTES) - VRAM_BASE
        };
        let lo = self.vram[(offset + tile_row as u16 * 2) as usize];
        let hi = self.vram[(offset + tile_row as u16 * 2 + 1) as usize];
        let color_idx = ((hi >> (7 - tile_col)) & 1) << 1 | ((lo >> (7 - tile_col)) & 1); 
        ((self.bgp >> (color_idx * 2)) & 0x03) as u8
    }  

    pub fn draw_line(&mut self) {
        // Background
        let map_base = if self.lcdc & LCDC_BG_TILE_MAP == 0 { TILE_MAP_A } else { TILE_MAP_B } as u16;
        let y = self.ly.wrapping_add(self.scy);
        let tile_map_row = (y / 8) as u16;
        let tile_row = y % 8;
        for d in 0..SCREEN_W as u8 {
            let x = d.wrapping_add(self.scx);
            let tile_map_col = (x / 8) as u16;
            let tile_col = x % 8;
            let tile_index = self.vram[((map_base + tile_map_row * TILE_MAP_SIZE + tile_map_col) - VRAM_BASE) as usize];
            let tile_color = self.get_tile_color(tile_index, tile_row, tile_col);
            self.frame_buffer[(self.ly as u16 * SCREEN_W as u16 + d as u16) as usize] = tile_color;
        }
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

    fn render_tile_ascii(ppu: &PPU, tile_index: u8) -> [String; 8] {
        const PALETTE_CHARS: [char; 4] = [' ', '.', 'º', '#'];
        std::array::from_fn(|row| {
            (0..8)
                .map(|col| {
                    let shade = ppu.get_tile_color(tile_index, row as u8, col as u8);
                    PALETTE_CHARS[shade as usize]
                })
                .collect()
        })
    }

    #[test]
    fn test_tile_decode_and_ascii_render() {
        let mut ppu = PPU::new();
        // Enable LCD, unsigned tile addressing (0x8000), and identity BGP (0b11_10_01_00 = 0xE4)
        ppu.set_lcdc(LCDC_LCDPPU_ENABLE | LCDC_BG_WINDOW_TILES | LCDC_BG_WINDOW_ENABLE);
        ppu.set_bgp(0xE4);

        // 8x8 tile covering all 4 color IDs (0=' ', 1='.', 2='º', 3='#')
        // Row format: [lo_byte, hi_byte]
        let tile_data: [u8; 16] = [
            0b0011_1100, 0b0000_0000, // Row 0: "  ....  "
            0b0100_0010, 0b0011_1100, // Row 1: " .ºººº. "
            0b1010_0101, 0b0000_0000, // Row 2: ". .  . ."
            0b1000_0001, 0b1000_0001, // Row 3: "#      #"
            0b1000_0001, 0b1011_1101, // Row 4: "# ºººº #"
            0b1010_0101, 0b0010_0100, // Row 5: ". #  # ."
            0b0100_0010, 0b0011_1100, // Row 6: " .ºººº. "
            0b0011_1100, 0b0011_1100, // Row 7: "  ####  "
        ];

        // Write tile at index 1 (VRAM offset 0x0010..0x001F)
        for (i, &byte) in tile_data.iter().enumerate() {
            ppu.write_vram(0x0010 + i as u16, byte);
        }

        let lines = render_tile_ascii(&ppu, 1);

        assert_eq!(lines[0], "  ....  ");
        assert_eq!(lines[1], " .ºººº. ");
        assert_eq!(lines[2], ". .  . .");
        assert_eq!(lines[3], "#      #");
        assert_eq!(lines[4], "# ºººº #");
        assert_eq!(lines[5], ". #  # .");
        assert_eq!(lines[6], " .ºººº. ");
        assert_eq!(lines[7], "  ####  ");
    }

    #[test]
    fn test_tile_signed_addressing_mode_0x8800_boundaries() {
        let mut ppu = PPU::new();
        // LCDC bit 4 = 0 -> Base 0x9000 with signed i8 index
        ppu.set_lcdc(LCDC_LCDPPU_ENABLE | LCDC_BG_WINDOW_ENABLE);
        ppu.set_bgp(0xE4);

        // Tile index 0x00 (0) -> 0x9000 (VRAM offset 0x1000) -> color ID 1 ('.')
        ppu.write_vram(0x1000, 0xFF);
        ppu.write_vram(0x1001, 0x00);

        // Tile index 0x7F (+127) -> 0x9000 + 2032 = 0x97F0 (VRAM offset 0x17F0) -> color ID 3 ('#')
        ppu.write_vram(0x17F0, 0xFF);
        ppu.write_vram(0x17F1, 0xFF);

        // Tile index 0xFF (-1) -> 0x9000 - 16 = 0x8FF0 (VRAM offset 0x0FF0) -> color ID 2 ('º')
        ppu.write_vram(0x0FF0, 0x00);
        ppu.write_vram(0x0FF1, 0xFF);

        // Tile index 0x80 (-128) -> 0x9000 - 2048 = 0x8800 (VRAM offset 0x0800) -> color ID 0 (' ')
        ppu.write_vram(0x0800, 0x00);
        ppu.write_vram(0x0801, 0x00);

        assert_eq!(render_tile_ascii(&ppu, 0x00)[0], "........");
        assert_eq!(render_tile_ascii(&ppu, 0x7F)[0], "########");
        assert_eq!(render_tile_ascii(&ppu, 0xFF)[0], "ºººººººº");
        assert_eq!(render_tile_ascii(&ppu, 0x80)[0], "        ");
    }

    #[test]
    fn test_tile_bgp_palette_remapping() {
        let mut ppu = PPU::new();
        ppu.set_lcdc(LCDC_LCDPPU_ENABLE | LCDC_BG_WINDOW_TILES | LCDC_BG_WINDOW_ENABLE);

        // Row 0 of tile 0 has pixels with color IDs: [0, 1, 2, 3, 0, 1, 2, 3]
        // lo = 0b0101_0101 (0x55), hi = 0b0011_0011 (0x33)
        ppu.write_vram(0x0000, 0x55);
        ppu.write_vram(0x0001, 0x33);

        // Normal palette: ID 0->0 (' '), ID 1->1 ('.'), ID 2->2 ('º'), ID 3->3 ('#')
        ppu.set_bgp(0b11_10_01_00);
        assert_eq!(render_tile_ascii(&ppu, 0)[0], " .º# .º#");

        // Inverted palette: ID 0->3 ('#'), ID 1->2 ('º'), ID 2->1 ('.'), ID 3->0 (' ')
        ppu.set_bgp(0b00_01_10_11);
        assert_eq!(render_tile_ascii(&ppu, 0)[0], "#º. #º. ");
    }

    #[test]
    fn test_draw_line_bg_map_framebuffer() {
        let mut ppu = PPU::new();
        // LCD ON, Unsigned Tile Data (0x8000), BG Map A (0x9800), BG Enable
        ppu.set_lcdc(LCDC_LCDPPU_ENABLE | LCDC_BG_WINDOW_TILES | LCDC_BG_WINDOW_ENABLE);
        ppu.set_bgp(0xE4);

        // Tile 1: decorative frame pattern
        let frame_tile: [u8; 16] = [
            0xFF, 0xFF, // ########
            0x81, 0x81, // #      #
            0xBD, 0x81, // # .... #
            0xA5, 0x99, // # .ºº. #
            0xA5, 0x99, // # .ºº. #
            0xBD, 0x81, // # .... #
            0x81, 0x81, // #      #
            0xFF, 0xFF, // ########
        ];
        for (i, &b) in frame_tile.iter().enumerate() {
            ppu.write_vram(0x0010 + i as u16, b);
        }

        // Place Tile 1 at (col=0, row=0) and (col=1, row=0) of BG Tile Map A (offset 0x1800)
        ppu.write_vram(0x1800, 1);
        ppu.write_vram(0x1801, 1);

        // Render first 8 scanlines
        for _ in 0..(8 * LINE_CYCLES as usize) {
            ppu.tick(1);
        }

        const PALETTE_CHARS: [char; 4] = [' ', '.', 'º', '#'];
        let fb = ppu.frame_buffer();

        // Verify both col=0 (0..8) and col=1 (8..16) match frame_tile
        let row0_col0: String = (0..8).map(|x| PALETTE_CHARS[fb[x] as usize]).collect();
        let row0_col1: String = (8..16).map(|x| PALETTE_CHARS[fb[x] as usize]).collect();
        let row3_col0: String = (0..8)
            .map(|x| PALETTE_CHARS[fb[3 * SCREEN_W as usize + x] as usize])
            .collect();
        let row3_col1: String = (8..16)
            .map(|x| PALETTE_CHARS[fb[3 * SCREEN_W as usize + x] as usize])
            .collect();

        assert_eq!(row0_col0, "########");
        assert_eq!(row0_col1, "########");
        assert_eq!(row3_col0, "# .ºº. #");
        assert_eq!(row3_col1, "# .ºº. #");
    }

    #[test]
    fn test_draw_line_bg_map_columns_and_scroll() {
        let mut ppu = PPU::new();
        // LCD ON, Unsigned Tile Data (0x8000), BG Map A (0x9800), BG Enable
        ppu.set_lcdc(LCDC_LCDPPU_ENABLE | LCDC_BG_WINDOW_TILES | LCDC_BG_WINDOW_ENABLE);
        ppu.set_bgp(0xE4);

        // Tile 1: solid '#' (color ID 3)
        for i in 0..16 {
            ppu.write_vram(0x0010 + i, 0xFF);
        }

        // Tile 2: solid '.' (color ID 1: lo = 0xFF, hi = 0x00)
        for row in 0..8 {
            ppu.write_vram(0x0020 + row * 2, 0xFF);
            ppu.write_vram(0x0020 + row * 2 + 1, 0x00);
        }

        // Tile 3: solid 'º' (color ID 2: lo = 0x00, hi = 0xFF)
        for row in 0..8 {
            ppu.write_vram(0x0030 + row * 2, 0x00);
            ppu.write_vram(0x0030 + row * 2 + 1, 0xFF);
        }

        // BG Tile Map A (offset 0x1800):
        // (col=0, row=0) = Tile 1 ('#'), (col=1, row=0) = Tile 2 ('.'), (col=0, row=1) = Tile 3 ('º')
        ppu.write_vram(0x1800, 1);
        ppu.write_vram(0x1801, 2);
        ppu.write_vram(0x1800 + TILE_MAP_SIZE, 3);

        const PALETTE_CHARS: [char; 4] = [' ', '.', 'º', '#'];
        let read_line0_slice = |ppu: &PPU, start: usize, end: usize| -> String {
            let fb = ppu.frame_buffer();
            (start..end)
                .map(|x| PALETTE_CHARS[fb[x] as usize])
                .collect()
        };

        // Case 1: SCX = 0, SCY = 0 -> cols 0..8 show Tile 1 ('#'), cols 8..16 show Tile 2 ('.')
        ppu.set_scx(0);
        ppu.set_scy(0);
        ppu.draw_line();
        assert_eq!(read_line0_slice(&ppu, 0, 8), "########");
        assert_eq!(read_line0_slice(&ppu, 8, 16), "........");

        // Case 2: SCX = 8, SCY = 0 -> shifts horizontally to (col=1, row=0), showing Tile 2 ('.')
        ppu.set_scx(8);
        ppu.set_scy(0);
        ppu.draw_line();
        assert_eq!(read_line0_slice(&ppu, 0, 8), "........");
        assert_eq!(read_line0_slice(&ppu, 8, 16), "        ");

        // Case 3: SCX = 0, SCY = 8 -> shifts vertically to (col=0, row=1), showing Tile 3 ('º')
        ppu.set_scx(0);
        ppu.set_scy(8);
        ppu.draw_line();
        assert_eq!(read_line0_slice(&ppu, 0, 8), "ºººººººº");
        assert_eq!(read_line0_slice(&ppu, 8, 16), "        ");
    }

    #[test]
    fn test_draw_line_bg_tile_map_b_selection() {
        let mut ppu = PPU::new();
        ppu.set_bgp(0xE4);

        // Tile 1: solid '#' (color ID 3)
        for i in 0..16 {
            ppu.write_vram(0x0010 + i, 0xFF);
        }

        // Tile 2: solid '.' (color ID 1)
        for row in 0..8 {
            ppu.write_vram(0x0020 + row * 2, 0xFF);
            ppu.write_vram(0x0020 + row * 2 + 1, 0x00);
        }

        // Map A (0x9800 -> VRAM offset 0x1800) uses Tile 1; Map B (0x9C00 -> VRAM offset 0x1C00) uses Tile 2
        ppu.write_vram(0x1800, 1);
        ppu.write_vram(0x1C00, 2);

        const PALETTE_CHARS: [char; 4] = [' ', '.', 'º', '#'];
        let read_line0_first8 = |ppu: &PPU| -> String {
            let fb = ppu.frame_buffer();
            (0..8).map(|x| PALETTE_CHARS[fb[x] as usize]).collect()
        };

        // LCDC bit 3 = 0 -> selects Map A (0x9800)
        ppu.set_lcdc(LCDC_LCDPPU_ENABLE | LCDC_BG_WINDOW_TILES | LCDC_BG_WINDOW_ENABLE);
        ppu.draw_line();
        assert_eq!(read_line0_first8(&ppu), "########");

        // LCDC bit 3 = 1 -> selects Map B (0x9C00)
        ppu.set_lcdc(
            LCDC_LCDPPU_ENABLE
                | LCDC_BG_WINDOW_TILES
                | LCDC_BG_TILE_MAP
                | LCDC_BG_WINDOW_ENABLE,
        );
        ppu.draw_line();
        assert_eq!(read_line0_first8(&ppu), "........");
    }

    

}