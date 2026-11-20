use super::Registers;
use super::Bus;

pub struct CPU {
    pub registers: Registers,
    pub bus: Bus
}

impl CPU {

    pub fn new(bus: Bus) -> CPU {
        CPU{
            registers: Registers::new(),
            bus
        }
    }

    fn fetch8(&mut self) -> u8 {
        let fetched = self.bus.read(self.registers.pc());
        self.registers.inc_pc(1);
        fetched
    }

    fn fetch16(&mut self) -> u16 {
        let fetched = self.bus.read16(self.registers.pc());
        self.registers.inc_pc(2);
        fetched
    }


    fn get_r(&self, r: u8) -> u8 {
        match r {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.bus.read(self.registers.hl()),
            7 => self.registers.a,
            _ => panic!("Invalid register read: {:#04X}", r)
        }
    }

    fn set_r(&mut self, r: u8, value: u8) {
        match r {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => self.bus.write(self.registers.hl(), value),
            7 => self.registers.a = value,
            _ => panic!("Invalid register write: {:#04X}", r)
        }
    }

    pub fn step(&mut self) -> u8 {
        let pc = self.registers.pc();
        let opcode = self.fetch8();
        let y = opcode >> 3 & 7;
        let z = opcode & 7;
        match opcode{
            // NOP
            0x00 => 4,

            // 
            0x01..=0x3F => {
                if z == 0b110 {
                    let data = self.fetch8();
                    self.set_r(y, data);
                    if y == 6 { 12 } else { 8 }
                }else{
                    panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                }
            },

            // 8 bit transfer
            0x40..=0x7F => {
                // Any LD r, r' (including HL)
                if opcode == 0x76{
                    panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                }

                self.set_r(y, self.get_r(z));
                // If HL have been read or written it takes 8 cycles, else, 4 
                if y == 6 || z == 6 { 8 } else { 4 }
            },


            0x80..=0xBF => {
                panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
            },


            0xC0..=0xFF => {
                if opcode == 0xC3 {
                    let addr = self.fetch16();
                    self.registers.set_pc(addr);
                    16
                }else{
                    panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                }
            }
        }
    }

}

#[cfg(test)]
mod test {
use super::*;

    fn load_program(cpu: &mut CPU, base: u16, program: &[u8]){
        for (i, &byte) in program.iter().enumerate() {
            cpu.bus.write(base.wrapping_add(i as u16), byte);
        }
    }

    #[test]
    fn test_program_a() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        /*  Test the folloging program
            LD B, 0x42 (B = 0x42; PC = 0xC002)
            LD C, B    (C = 0x42, PC = 0xC003)
            JP 0x1234  (PC = 0x1234)
            LD H, 0x80 (H = 0x80; PC = 0xC008)
            LD L, 0x00 (L = 0x00; PC = 0xC00A)
            LD (HL), B ((HL) = 0x42; PC = 0xC00B)
            LD E, (HL) (E = 0x42; PC = 0xC00C)
         */
        load_program(&mut cpu, 0xC000, &[
            0x06, 0x42,
            0x48, 
            0xC3, 0x34, 0x12,
            0x26, 0x80,
            0x2E, 0x00, 
            0x70, 0x5E]);
        cpu.registers.set_pc(0xC000);
        assert_eq!(cpu.step(), 8); // LD B, 0x42
        assert_eq!(cpu.registers.b, 0x42);
        assert_eq!(cpu.registers.pc, 0xC002);
        assert_eq!(cpu.step(), 4); // LD C, B
        assert_eq!(cpu.registers.c, 0x42);
        assert_eq!(cpu.registers.pc, 0xC003);
        assert_eq!(cpu.step(), 16); // JP 0x1234
        assert_eq!(cpu.registers.pc, 0x1234);
        cpu.registers.set_pc(0xC006);
        assert_eq!(cpu.step(), 8); // LD H, 0x80
        assert_eq!(cpu.registers.h, 0x80);
        assert_eq!(cpu.registers.pc, 0xC008);
        assert_eq!(cpu.step(), 8); // LD L, 0x00
        assert_eq!(cpu.registers.l, 0x00);
        assert_eq!(cpu.registers.pc, 0xC00A);
        assert_eq!(cpu.step(), 8); // LD (HL), B
        assert_eq!(cpu.bus.read(0x8000), 0x42);
        assert_eq!(cpu.registers.pc, 0xC00B);
        assert_eq!(cpu.step(), 8); // LD E, (HL)
        assert_eq!(cpu.registers.e, 0x42);
        assert_eq!(cpu.registers.pc, 0xC00C);
    }

}