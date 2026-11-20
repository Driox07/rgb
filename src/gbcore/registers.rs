pub const FLAG_C: u8 = 4;
pub const FLAG_H: u8 = 5;
pub const FLAG_N: u8 = 6;
pub const FLAG_Z: u8 = 7;

pub struct Registers {
    pub a:  u8,
    pub f:  u8,
    pub b:  u8,
    pub c:  u8,
    pub d:  u8,
    pub e:  u8,
    pub h:  u8,
    pub l:  u8,
    pub sp: u16,
    pub pc: u16
}

impl Registers {
    
    pub fn new() -> Registers {
        Registers {
            a:  0x01,
            f:  0xB0,
            b:  0x00,
            c:  0x13,
            d:  0x00,
            e:  0xD8,
            h:  0x01,
            l:  0x4D,
            sp: 0xFFFE,
            pc: 0x0100
        }
    }

    pub fn af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f])
    }

    pub fn set_af(&mut self, value: u16) -> () {
        let masked = value & 0xFFF0;
        self.a = (masked >> 8) as u8;
        self.f = (masked & 0xFF) as u8;
    }

    pub fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    pub fn set_bc(&mut self, value: u16) -> () {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    pub fn set_de(&mut self, value: u16) -> () {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    pub fn set_hl(&mut self, value: u16) -> () {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    pub fn sp(&self) -> u16 {
        self.sp
    }

    pub fn set_sp(&mut self, value: u16) -> () {
        self.sp = value;
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn set_pc(&mut self, value: u16) -> () {
        self.pc = value;
    }

    pub fn inc_pc(&mut self, value: u16) -> () {
        self.pc = self.pc.wrapping_add(value);
    }

    pub fn flag(&self, f: u8) -> bool {
        self.f & (1 << f) != 0
    }

    pub fn set_flag(&mut self, f: u8, value: bool) -> (){
        if value{
            self.f = self.f | (1 << f);
        }
        else{
            self.f = self.f & !(1 << f);
        }
    }

}

#[cfg(test)]
mod tests {
use super::*;

    #[test]
    fn set_registers(){
        let mut regs = Registers::new();
        regs.set_af(0xF00F);
        assert_eq!(regs.a, 0xF0);
        assert_eq!(regs.f, 0x00);
        regs.set_bc(0x1234);
        assert_eq!(regs.b, 0x12);
        assert_eq!(regs.c, 0x34);
        regs.set_de(0xABCD);
        assert_eq!(regs.d, 0xAB);
        assert_eq!(regs.e, 0xCD);
    }

    #[test]
    fn set_registers_whole() {
        let mut regs = Registers::new();
        regs.set_af(0xF00F);
        assert_eq!(regs.af(), 0xF000);
        regs.set_bc(0x1234);
        assert_eq!(regs.bc(), 0x1234);
        regs.set_de(0xABCD);
        assert_eq!(regs.de(), 0xABCD);
        regs.set_sp(0xF00F);
        assert_eq!(regs.sp, 0xF00F);
        regs.set_pc(0xF00F);
        assert_eq!(regs.pc, 0xF00F);
    }

    #[test]
    fn flag_check() {
        let mut regs = Registers::new();
        assert!(regs.flag(FLAG_Z));
        assert!(regs.flag(FLAG_H));
        assert!(!regs.flag(FLAG_N));
        assert!(regs.flag(FLAG_C));
        regs.set_flag(FLAG_Z, false);
    assert!(!regs.flag(FLAG_Z));
        regs.set_flag(FLAG_H, false);
        assert!(!regs.flag(FLAG_H));
        regs.set_flag(FLAG_N, true);
        assert!(regs.flag(FLAG_N));
        regs.set_flag(FLAG_C, false);
        assert!(!regs.flag(FLAG_C));
    }

    #[test]
    fn flag_integrity() {
        let mut regs = Registers::new();
        for i in 4..8 {
            for f in 4..8 {
                regs.set_flag(f, true);
            }
            for f in 4..8 {
                assert!(regs.flag(f));
            }
            regs.set_flag(i, false);
            for f in 4..8 {
                if f == i {
                    assert!(!regs.flag(f));
                }else{
                    assert!(regs.flag(f));
                }
            }
        }    
    }
}
