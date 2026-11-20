use crate::gbcore::registers;

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

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.registers.set_flag(registers::FLAG_Z, z);
        self.registers.set_flag(registers::FLAG_N, n);
        self.registers.set_flag(registers::FLAG_H, h);
        self.registers.set_flag(registers::FLAG_C, c);
    }

    fn alu(&mut self, op: u8, value: u8) {
        match op {
            0 => { // ADD
                let (res, c) = self.registers.a.overflowing_add(value);
                self.set_flags(res == 0, false, ((self.registers.a&0x0F) + (value&0x0F)) > 0x0F, c);
                self.registers.a = res;
            },
            1 => { // ADC
                let carrying = if self.registers.flag(registers::FLAG_C) {1} else {0};
                let (preres, c1) = value.overflowing_add(carrying);
                let (res, c2) = self.registers.a.overflowing_add(preres);
                self.set_flags(res == 0, false, (self.registers.a&0x0F) + (value&0x0F) + carrying > 0x0F, c1 || c2);
                self.registers.a = res;
            },
            2 => { // SUB
                let (res, c) = self.registers.a.overflowing_sub(value);
                self.set_flags(res == 0, true, (self.registers.a&0x0F) < (value&0x0F), c);
                self.registers.a = res;
            },
            3 => { // SBC
                let carrying = if self.registers.flag(registers::FLAG_C) {1} else {0};
                let (preres, c1) = value.overflowing_add(carrying);
                let (res, c2) = self.registers.a.overflowing_sub(preres);
                self.set_flags(res == 0, true, (self.registers.a&0x0F) < (value&0x0F) + carrying, c1 || c2);
                self.registers.a = res;
            },
            4 => { // AND
                let res = self.registers.a & value;
                self.set_flags(res == 0, false, true, false);
                self.registers.a = res;
            },
            5..=6 => { // XOR & OR
                let res = if op == 5 { self.registers.a ^ value } else { self.registers.a | value };
                self.set_flags(res == 0, false, false, false);
                self.registers.a = res;
            }, 
            7 => { // CP
                let (res, c) = self.registers.a.overflowing_sub(value);
                self.set_flags(res == 0, true, (self.registers.a&0x0F) < (value&0x0F), c);
            },
            _ => panic!("Invalid ALU operation received: {:#04X}", op)
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

            // 8b ALU Operations
            0x80..=0xBF => {
                self.alu(y, self.get_r(z));
                if z == 6 { 8 } else { 4 }
            },


            0xC0..=0xFF => {
                if opcode == 0xC3 {
                    let addr = self.fetch16();
                    self.registers.set_pc(addr);
                    16
                }else if z == 0b110 {
                    let value = self.fetch8();
                    self.alu(y, value);
                    8
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

    #[test]
    fn test_alu_imm() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        /*  Test the following program:
            LD A, 0x0F (A = 0x0F; PC = 0xC002)
            ADD 0x01   (A = 0x10; Z=0, N=0, H=1, C=0; PC = 0xC004)
            LD A, 0xFF (A = 0xFF; PC = 0xC006)
            ADD 0x01   (A = 0x00; Z=1, N=0, H=1, C=1; PC = 0xC008)
            LD A, 0x10 (A = 0x10; PC = 0xC00A)
            SUB 0x01   (A = 0x0F; Z=0, N=1, H=1, C=0; PC = 0xC00C)
            LD A, 0x05 (A = 0x05; PC = 0xC00E)
            SUB 0x05   (A = 0x00; Z=1, N=1, H=0, C=0; PC = 0xC010)
            LD A, 0x05 (A = 0x05; PC = 0xC012)
            CP 0x05    (A = 0x05; Z=1, N=1, H=0, C=0; PC = 0xC014)
        */
        load_program(&mut cpu, 0xC000, &[
            0x3E, 0x0F, // LD A, 0x0F
            0xC6, 0x01, // ADD A, 0x01
            0x3E, 0xFF, // LD A, 0xFF
            0xC6, 0x01, // ADD A, 0x01
            0x3E, 0x10, // LD A, 0x10
            0xD6, 0x01, // SUB 0x01
            0x3E, 0x05, // LD A, 0x05
            0xD6, 0x05, // SUB 0x05
            0x3E, 0x05, // LD A, 0x05
            0xFE, 0x05, // CP 0x05
        ]);
        cpu.registers.set_pc(0xC000);

        // LD A, 0x0F
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x0F);
        assert_eq!(cpu.registers.pc, 0xC002);

        // ADD 0x01 -> Half-Carry (0x0F + 0x01 = 0x10)
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x10);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC004);

        // LD A, 0xFF
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0xFF);
        assert_eq!(cpu.registers.pc, 0xC006);

        // ADD 0x01 -> Zero + Half-Carry + Carry (0xFF + 0x01 = 0x00)
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC008);

        // LD A, 0x10
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x10);
        assert_eq!(cpu.registers.pc, 0xC00A);

        // SUB 0x01 -> Half-Carry borrow (0x10 - 0x01 = 0x0F)
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x0F);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC00C);

        // LD A, 0x05
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x05);
        assert_eq!(cpu.registers.pc, 0xC00E);

        // SUB 0x05 -> Zero + Subtract (0x05 - 0x05 = 0x00)
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC010);

        // LD A, 0x05
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x05);
        assert_eq!(cpu.registers.pc, 0xC012);

        // CP 0x05 -> Mismos flags que SUB 0x05, pero A sigue valiendo 0x05
        assert_eq!(cpu.step(), 8);
        assert_eq!(cpu.registers.a, 0x05);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC014);
    }

    #[test]
    fn test_alu_reg() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        /*  Test the following program:
            LD A, 0x0F  (A = 0x0F; PC = 0xC002) [8 cycles]
            LD B, 0x01  (B = 0x01; PC = 0xC004) [8 cycles]
            ADD A, B    (A = 0x10; Z=0, N=0, H=1, C=0; PC = 0xC005) [4 cycles]
            LD C, 0xFF  (C = 0xFF; PC = 0xC007) [8 cycles]
            ADD A, C    (A = 0x0F; Z=0, N=0, H=1, C=1; PC = 0xC008) [4 cycles]
            ADC A, B    (A = 0x11; Z=0, N=0, H=1, C=0; PC = 0xC009) [4 cycles] -> 0x0F + 0x01 + C(1) = 0x11
            LD D, 0x02  (D = 0x02; PC = 0xC00B) [8 cycles]
            SUB D       (A = 0x0F; Z=0, N=1, H=1, C=0; PC = 0xC00C) [4 cycles] -> 0x11 - 0x02 = 0x0F
            LD E, 0x20  (E = 0x20; PC = 0xC00E) [8 cycles]
            SUB E       (A = 0xEF; Z=0, N=1, H=0, C=1; PC = 0xC00F) [4 cycles] -> 0x0F - 0x20 = 0xEF (underflow -> C=1)
            SBC A, B    (A = 0xED; Z=0, N=1, H=0, C=0; PC = 0xC010) [4 cycles] -> 0xEF - 0x01 - C(1) = 0xED
            AND D       (A = 0x00; Z=1, N=0, H=1, C=0; PC = 0xC011) [4 cycles] -> 0xED & 0x02 = 0x00 (H siempre 1 en AND)
            OR B        (A = 0x01; Z=0, N=0, H=0, C=0; PC = 0xC012) [4 cycles] -> 0x00 | 0x01 = 0x01
            XOR B       (A = 0x00; Z=1, N=0, H=0, C=0; PC = 0xC013) [4 cycles] -> 0x01 ^ 0x01 = 0x00
            CP B        (A = 0x00; Z=0, N=1, H=1, C=1; PC = 0xC014) [4 cycles] -> 0x00 - 0x01 (A no cambia)
        */
        load_program(&mut cpu, 0xC000, &[
            0x3E, 0x0F, // LD A, 0x0F
            0x06, 0x01, // LD B, 0x01
            0x80,       // ADD A, B
            0x0E, 0xFF, // LD C, 0xFF
            0x81,       // ADD A, C
            0x88,       // ADC A, B
            0x16, 0x02, // LD D, 0x02
            0x92,       // SUB D
            0x1E, 0x20, // LD E, 0x20
            0x93,       // SUB E
            0x98,       // SBC A, B
            0xA2,       // AND D
            0xB0,       // OR B
            0xA8,       // XOR B
            0xB8,       // CP B
        ]);
        cpu.registers.set_pc(0xC000);

        // A = 0x0F, B = 0x01
        assert_eq!(cpu.step(), 8); // LD A, 0x0F
        assert_eq!(cpu.registers.a, 0x0F);
        assert_eq!(cpu.step(), 8); // LD B, 0x01
        assert_eq!(cpu.registers.b, 0x01);
        assert_eq!(cpu.registers.pc, 0xC004);

        // ADD A, B -> 0x0F + 0x01 = 0x10 (H=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x10);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC005);

        // LD C, 0xFF
        assert_eq!(cpu.step(), 8);

        // ADD A, C -> 0x10 + 0xFF = 0x0F (H=1, C=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x0F);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC008);

        // ADC A, B -> 0x0F + 0x01 + C(1) = 0x11 (H=1, C=0)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x11);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC009);

        // LD D, 0x02
        assert_eq!(cpu.step(), 8);

        // SUB D -> 0x11 - 0x02 = 0x0F (N=1, H=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x0F);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC00C);

        // LD E, 0x20
        assert_eq!(cpu.step(), 8);

        // SUB E -> 0x0F - 0x20 = 0xEF (N=1, H=0, C=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0xEF);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC00F);

        // SBC A, B -> 0xEF - 0x01 - C(1) = 0xED (N=1, H=0, C=0)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0xED);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC010);

        // AND D -> 0xED (0b1110_1101) & 0x02 (0b0000_0010) = 0x00 (Z=1, H=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC011);

        // OR B -> 0x00 | 0x01 = 0x01 (todos los flags a 0)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x01);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC012);

        // XOR B -> 0x01 ^ 0x01 = 0x00 (Z=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC013);

        // CP B -> 0x00 - 0x01 (A sigue en 0x00; Z=0, N=1, H=1, C=1)
        assert_eq!(cpu.step(), 4);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC014);
    }

    #[test]
    fn test_alu_adc_sbc_hl() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        /*  Test the following program:
            LD H, 0xC1      (H = 0xC1; PC = 0xC002)
            LD L, 0x00      (L = 0x00; HL = 0xC100; PC = 0xC004)
            LD A, 0x25      (A = 0x25; PC = 0xC006)
            LD (HL), A      ((0xC100) = 0x25; PC = 0xC007)
            LD A, 0xFF      (A = 0xFF; PC = 0xC009)
            LD B, 0x01      (B = 0x01; PC = 0xC00B)
            ADD A, B        (A = 0x00; Z=1, N=0, H=1, C=1; PC = 0xC00C)
            ADC A, (HL)     (A = 0x26; Z=0, N=0, H=0, C=0; PC = 0xC00D)
            LD A, 0xFF      (A = 0xFF; PC = 0xC00F)
            LD C, 0x00      (C = 0x00; PC = 0xC011)
            ADD A, B        (A = 0x00; Z=1, N=0, H=1, C=1; PC = 0xC012)
            LD A, 0xFF      (A = 0xFF; PC = 0xC014)
            ADC A, C        (A = 0x00; Z=1, N=0, H=1, C=1; PC = 0xC015)
            SBC A, (HL)     (A = 0xDA; Z=0, N=1, H=1, C=1; PC = 0xC016)
            SBC A, B        (A = 0xD8; Z=0, N=1, H=0, C=0; PC = 0xC017)
            LD A, 0x10      (A = 0x10; PC = 0xC019)
            CP (HL)         (A = 0x10; Z=0, N=1, H=1, C=1; PC = 0xC01A)
        */
        load_program(&mut cpu, 0xC000, &[
            0x26, 0xC1,
            0x2E, 0x00,
            0x3E, 0x25,
            0x77,
            0x3E, 0xFF,
            0x06, 0x01,
            0x80,
            0x8E,
            0x3E, 0xFF,
            0x0E, 0x00,
            0x80,
            0x3E, 0xFF,
            0x89,
            0x9E,
            0x98,
            0x3E, 0x10,
            0xBE,
        ]);
        cpu.registers.set_pc(0xC000);

        assert_eq!(cpu.step(), 8); // LD H, 0xC1
        assert_eq!(cpu.step(), 8); // LD L, 0x00
        assert_eq!(cpu.step(), 8); // LD A, 0x25
        assert_eq!(cpu.step(), 8); // LD (HL), A
        assert_eq!(cpu.bus.read(0xC100), 0x25);
        assert_eq!(cpu.registers.pc, 0xC007);

        assert_eq!(cpu.step(), 8); // LD A, 0xFF
        assert_eq!(cpu.step(), 8); // LD B, 0x01
        assert_eq!(cpu.step(), 4); // ADD A, B -> A=0x00, Z=1, N=0, H=1, C=1
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC00C);

        assert_eq!(cpu.step(), 8); // ADC A, (HL) -> A=0x26, Z=0, N=0, H=0, C=0
        assert_eq!(cpu.registers.a, 0x26);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC00D);

        assert_eq!(cpu.step(), 8); // LD A, 0xFF
        assert_eq!(cpu.step(), 8); // LD C, 0x00
        assert_eq!(cpu.step(), 4); // ADD A, B -> C=1
        assert_eq!(cpu.step(), 8); // LD A, 0xFF
        assert_eq!(cpu.step(), 4); // ADC A, C -> A=0x00, Z=1, N=0, H=1, C=1
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC015);

        assert_eq!(cpu.step(), 8); // SBC A, (HL) -> A=0xDA, Z=0, N=1, H=1, C=1
        assert_eq!(cpu.registers.a, 0xDA);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC016);

        assert_eq!(cpu.step(), 4); // SBC A, B -> A=0xD8, Z=0, N=1, H=0, C=0
        assert_eq!(cpu.registers.a, 0xD8);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC017);

        assert_eq!(cpu.step(), 8); // LD A, 0x10
        assert_eq!(cpu.step(), 8); // CP (HL) -> A=0x10, Z=0, N=1, H=1, C=1
        assert_eq!(cpu.registers.a, 0x10);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC01A);
    }

}