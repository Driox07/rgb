pub const KEY_A: u8 = 0x01;
pub const KEY_B: u8 = 0x02;
pub const KEY_SELECT: u8 = 0x04;
pub const KEY_START: u8 = 0x08;
pub const KEY_RIGHT: u8 = 0x10;
pub const KEY_LEFT: u8 = 0x20;
pub const KEY_UP: u8 = 0x40;
pub const KEY_DOWN: u8 = 0x80;

pub struct Joypad {
    select: u8,
    keys: u8
}

impl Joypad {

    pub fn new() -> Joypad {
        Joypad {
            select: 0xCF,
            keys: 0xFF
        }
    }

    pub fn input(&self) -> u8 {
        let mut k = 0x0F;
        if self.select & 0x10 == 0 { k &= self.keys >> 4 };
        if self.select & 0x20 == 0 { k &= self.keys & 0x0F };
        0xC0 | self.select & 0x30 | k
    }

    pub fn set_input(&mut self, value: u8) {
        self.select &= !0x30;
        self.select |= value & 0x30;
    }

    pub fn set_key(&mut self, key: u8, value: bool){
        if value {
            self.keys &= !key;
        }else{
            self.keys |= key;
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_nothing_pressed() {
        let mut jp = Joypad::new();
        jp.set_input(0x20); // d-pad row
        assert_eq!(jp.input(), 0xEF);
        jp.set_input(0x10); // buttons row
        assert_eq!(jp.input(), 0xDF);
        jp.set_input(0x30); // no row selected
        assert_eq!(jp.input(), 0xFF);
    }

    #[test]
    fn test_joypad_rows_are_separated() {
        let mut jp = Joypad::new();
        jp.set_key(KEY_RIGHT, true);
        jp.set_key(KEY_START, true);

        // D-pad row: only Right (bit 0) is 0
        jp.set_input(0x20);
        assert_eq!(jp.input(), 0xEE);

        // Buttons row: only Start (bit 3) is 0
        jp.set_input(0x10);
        assert_eq!(jp.input(), 0xD7);

        // Releasing a key sets its bit back to 1
        jp.set_key(KEY_START, false);
        assert_eq!(jp.input(), 0xDF);
    }

    #[test]
    fn test_joypad_write_keeps_only_select_bits() {
        let mut jp = Joypad::new();
        jp.set_key(KEY_A, true);
        jp.set_input(0x1F); // low bits written by the game must be ignored
        assert_eq!(jp.input(), 0xDE);
    }
}