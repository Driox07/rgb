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

    fn push(&mut self, value: u16) {
        self.registers.set_sp(self.registers.sp().wrapping_sub(2));
        self.bus.write16(self.registers.sp(), value);
    }

    fn pop(&mut self) -> u16 {
        let val = self.bus.read16(self.registers.sp());
        self.registers.set_sp(self.registers.sp().wrapping_add(2));
        val
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

    fn get_rr(&self, rr: u8, sp: bool) -> u16 {
        match rr {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            3 => if sp { self.registers.sp() } else { self.registers.af() },
            _ => panic!("Invalid register 16 read: {:#04X}", rr)
        }
    }

    fn set_rr(&mut self, rr: u8, value: u16, sp: bool) {
        match rr {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            3 => if sp { self.registers.set_sp(value) } else { self.registers.set_af(value) },
            _ => panic!("Invalid register 16 read: {:#04X}", rr)
        }
    }

    fn get_mem_at_rr(&mut self, rr: u8) -> u8 {
        match rr {
            0 => self.bus.read(self.registers.bc()),
            1 => self.bus.read(self.registers.de()),
            2 => { 
                let val = self.bus.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                val
            },
            3 => {
                let val = self.bus.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                val
            },
            _ => panic!("Invalid register 16 memory read: {:#04X}", rr)
        }
    }

    fn set_mem_at_rr(&mut self, rr: u8, value: u8) {
        match rr {
            0 => self.bus.write(self.registers.bc(), value),
            1 => self.bus.write(self.registers.de(), value),
            2 => { 
                self.bus.write(self.registers.hl(), value);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
            },
            3 => {
                self.bus.write(self.registers.hl(), value);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
            },
            _ => panic!("Invalid register 16 memory read: {:#04X}", rr)
        }
    }

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.registers.set_flag(registers::FLAG_Z, z);
        self.registers.set_flag(registers::FLAG_N, n);
        self.registers.set_flag(registers::FLAG_H, h);
        self.registers.set_flag(registers::FLAG_C, c);
    }

    fn check_condition(&self, cond: u8) -> bool {
        match cond {
            0 => !self.registers.flag(registers::FLAG_Z),
            1 => self.registers.flag(registers::FLAG_Z),
            2 => !self.registers.flag(registers::FLAG_C),
            3 => self.registers.flag(registers::FLAG_C),
            _    => panic!("Invalid condition value!")
        }
    }

    fn alu(&mut self, op: u8, a: u8, b: u8) -> u8 {
        match op {
            0 => { // ADD
                let (res, c) = a.overflowing_add(b);
                self.set_flags(res == 0, false, ((a&0x0F) + (b&0x0F)) > 0x0F, c);
                res
            },
            1 => { // ADC
                let carrying = if self.registers.flag(registers::FLAG_C) {1} else {0};
                let (preres, c1) = b.overflowing_add(carrying);
                let (res, c2) = a.overflowing_add(preres);
                self.set_flags(res == 0, false, (a&0x0F) + (b&0x0F) + carrying > 0x0F, c1 || c2);
                res
            },
            2 => { // SUB
                let (res, c) = a.overflowing_sub(b);
                self.set_flags(res == 0, true, (a&0x0F) < (b&0x0F), c);
                res
            },
            3 => { // SBC
                let carrying = if self.registers.flag(registers::FLAG_C) {1} else {0};
                let (preres, c1) = b.overflowing_add(carrying);
                let (res, c2) = a.overflowing_sub(preres);
                self.set_flags(res == 0, true, (a&0x0F) < (b&0x0F) + carrying, c1 || c2);
                res
            },
            4 => { // AND
                let res = a & b;
                self.set_flags(res == 0, false, true, false);
                res
            },
            5..=6 => { // XOR & OR
                let res = if op == 5 { a ^ b } else { a | b };
                self.set_flags(res == 0, false, false, false);
                res
            }, 
            7 => { // CP
                let (res, c) = a.overflowing_sub(b);
                self.set_flags(res == 0, true, (self.registers.a&0x0F) < (b&0x0F), c);
                a
            },
            _ => panic!("Invalid ALU operation received: {:#04X}", op)
        }
    }

    pub fn step(&mut self) -> u8 {
        /*
        OPCODE PARTS
        ===============
        7 6 5 4 3 2 1 0
        --- ----- -----
        x   y     z
        */
        let pc = self.registers.pc();
        let opcode = self.fetch8();

        let y = opcode >> 3 & 7;
        let z = opcode & 7;
        let cond = opcode >> 3 & 3;

        match opcode{
            ..=0x3F => {
                match z {
                    0 => {
                        match y {
                            // NOP
                            0 => 4,

                            // LD (addr), SP
                            1 => {
                                let addr = self.fetch16();
                                self.bus.write16(addr, self.registers.sp());
                                20
                            },

                            // STOP
                            // 2 => 

                            // JR i8
                            3 => {
                                let value = self.fetch8() as i8;
                                self.registers.inc_pc_signed(value as i16);
                                12
                            },
                            
                            // JR cond, i8
                            4..=7 => { 
                                let value = self.fetch8() as i8;
                                if self.check_condition(cond) {
                                    self.registers.inc_pc_signed(value as i16);
                                    12
                                }else{
                                    8
                                }
                            },

                            _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                        }
                    },

                    // LD rr, u16
                    1 => {
                        let rr = y >> 1 & 3;
                        let value = self.fetch16();
                        self.set_rr(rr, value, true);
                        12
                    },

                    // LD (rr), A y LD A, (rr)
                    2 => {
                        // If y lsb == 0 -> ret to mem, else, mem to reg
                        let rr = y >> 1 & 3;
                        let ylsb = y & 1;
                        if ylsb == 0 {
                            self.set_mem_at_rr(rr, self.registers.a);
                        } else {
                            let value = self.get_mem_at_rr(rr);
                            self.registers.a = value;
                        }
                        8
                    },

                    // INC - DEC rr
                    3 => {
                        let rr = y >> 1 & 3;
                        let ylsb = y & 1;
                        if ylsb == 0 {
                            self.set_rr(rr, self.get_rr(rr,true).wrapping_add(1), true);
                        }else{
                            self.set_rr(rr, self.get_rr(rr, true).wrapping_sub(1), true);
                        }
                        8
                    },

                    // INC r
                    4 => {
                        let curr_c = self.registers.flag(registers::FLAG_C);
                        let res = self.alu(0, self.get_r(y), 1);
                        self.registers.set_flag(registers::FLAG_C, curr_c);
                        self.set_r(y, res);
                        if y == 6 { 12 } else { 4 }
                    },

                    // DEC r
                    5 => {
                        let curr_c = self.registers.flag(registers::FLAG_C);
                        let res = self.alu(2, self.get_r(y), 1);
                        self.registers.set_flag(registers::FLAG_C, curr_c);
                        self.set_r(y, res);
                        if y == 6 { 12 } else { 4 }
                    },

                    // LD r, u8
                    6 => {
                        let val = self.fetch8();
                        self.set_r(y, val);
                        if y == 6 { 12 } else { 8 }
                    }

                    _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)

                }   
            },

            0x40..=0x7F => {
                
                // HALT
                if opcode == 0x76{
                    panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                }

                // Any LD r, r'
                self.set_r(y, self.get_r(z));
                // If HL have been read or written it takes 8 cycles, else, 4 
                if y == 6 || z == 6 { 8 } else { 4 }
            },

            // 8b ALU Operations
            0x80..=0xBF => {
                let res = self.alu(y, self.registers.a, self.get_r(z));
                self.registers.a = res;
                if z == 6 { 8 } else { 4 }
            },


            0xC0..=0xFF => {
                match z {
                    // LD HL
                    0 => {

                        match y {
                            
                            // RET cc
                            0..=3 => {
                                if self.check_condition(cond){
                                    let stored_pc = self.pop();
                                    self.registers.set_pc(stored_pc);
                                    20
                                }else{
                                    8
                                }
                            },

                            // ADD SP, i8
                            // 5 => {

                            // },

                            // LDH
                            4 | 6 => {
                                let offset = self.fetch8();
                                let addr = 0xFF00 + offset as u16;
                                if y == 4{ // Reg to mem
                                    self.bus.write(addr, self.registers.a);
                                }else{ // Mem to reg
                                    self.registers.a = self.bus.read(addr);
                                }
                                12
                            },

                            // LD HL, SP + i8
                            // 7 => {

                            // },

                            _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)

                        }

                    },

                    1 => {
                        match y {

                            // RET
                            1 => {
                                let stored_pc = self.pop();
                                self.registers.set_pc(stored_pc);
                                16
                            },

                            // RETI
                            // 3 => {

                            // },

                            // POP
                            0 | 2 | 4 | 6 => {
                                let rr = y >> 1 & 3;
                                let stored = self.pop();
                                self.set_rr(rr, stored, false);
                                12
                            },

                            // JP HL
                            5 => {
                                self.registers.set_pc(self.registers.hl());
                                4
                            },

                            // LD SP, HL
                            // 7 => {

                            // },

                            _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                        }
                    },

                    2 => {
                        match y {
                            // JP CC
                            0..=3 => {
                                let addr = self.fetch16();
                                if self.check_condition(cond) {
                                    self.registers.set_pc(addr);
                                    16
                                }else{
                                    12
                                }
                            },

                            // LD (C), A / LD A, (C)
                            4 | 6 => {
                                if y == 4 { // Reg to mem
                                    self.bus.write(0xFF00 + self.registers.c as u16, self.registers.a);
                                }else{ // Mem to reg
                                    self.registers.a = self.bus.read(0xFF00 + self.registers.c as u16);
                                }
                                8
                            },

                            // LD (u16), A / LD A, (u16)
                            5 | 7 => {
                                let addr = self.fetch16();
                                if y == 5 { // reg to mem
                                    self.bus.write(addr, self.registers.a);
                                }else{ // mem to reg
                                    self.registers.a = self.bus.read(addr);
                                }
                                16
                            },

                            _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                        } 
                    },
                    
                    3 => {
                        match y {
                            // JP U16
                            0 => {
                                let addr = self.fetch16();
                                self.registers.set_pc(addr);
                                16
                            },

                            // Prefix CB
                            // 1 => {},

                            // DI
                            // 6 => {},

                            // EI
                            // 7 => {},

                            _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
                        }
                    },

                    // CALL CC, 
                    4 => {
                        let addr = self.fetch16();
                        if self.check_condition(cond) {
                            self.push(self.registers.pc());
                            self.registers.set_pc(addr);
                            24
                        }else{
                            12
                        }
                    },

                    5 => {
                        match y {
                            // CALL u16
                            1 => {
                                let addr = self.fetch16();
                                self.push(self.registers.pc());
                                self.registers.set_pc(addr);
                                24
                            },
                            // Push rr
                            0 | 2 | 4 | 6 => {
                                let rr = y >> 1 & 3;
                                let value = self.get_rr(rr, false);
                                self.push(value);
                                16
                            },
                            _ => panic!("Illegal opcode: {:#04X} at PC = {:#06X}", opcode, pc)
                        }
                    },

                    // ALU A, u8
                    6 => {
                        let value = self.fetch8();
                        let res = self.alu(y, self.registers.a, value);
                        self.registers.a = res;
                        8
                    },

                    // RST
                    // 7 => {},

                    _ => panic!("Not implemented: {:#04X} at PC = {:#06X}", opcode, pc)
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

        // CP B -> 0x00 - 0x01 (A still in 0x00; Z=0, N=1, H=1, C=1)
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

    #[test]
    fn test_control_flow_and_stack() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        /*
            0xC000: LD SP, 0xFFFE      (SP = 0xFFFE; PC = 0xC003)
            0xC003: LD BC, 0x12FF      (BC = 0x12FF; PC = 0xC006)
            0xC006: PUSH BC            (SP = 0xFFFC; (0xFFFD)=0x12, (0xFFFC)=0xFF; PC = 0xC007)
            0xC007: POP AF             (A = 0x12, F = 0xF0; SP = 0xFFFE; PC = 0xC008)
            0xC008: PUSH AF            (SP = 0xFFFC; (0xFFFD)=0x12, (0xFFFC)=0xF0; PC = 0xC009)
            0xC009: POP DE             (DE = 0x12F0; SP = 0xFFFE; PC = 0xC00A)
            0xC00A: LD HL, 0xC012      (HL = 0xC012; PC = 0xC00D)
            0xC00D: JR +2              (PC = 0xC011)
            0xC00F: LD A, 0x99         (skipped)
            0xC011: JP (HL)            (PC = 0xC012)
            0xC012: LD A, 0x05         (A = 0x05; PC = 0xC014)
            0xC014: SUB 0x05           (A = 0x00; Z=1, N=1, H=0, C=0; PC = 0xC016)
            0xC016: JR NZ, +2          (not taken; PC = 0xC018)
            0xC018: JR Z, +2           (taken; PC = 0xC01C)
            0xC01A: LD A, 0x99         (skipped)
            0xC01C: JR C, +2           (not taken; PC = 0xC01E)
            0xC01E: JR NC, +2          (taken; PC = 0xC022)
            0xC020: LD A, 0x99         (skipped)
            0xC022: JP NZ, 0xC050      (not taken; PC = 0xC025)
            0xC025: JP Z, 0xC02A       (taken; PC = 0xC02A)
            0xC028: LD A, 0x99         (skipped)
            0xC02A: JP C, 0xC050       (not taken; PC = 0xC02D)
            0xC02D: JP NC, 0xC032      (taken; PC = 0xC032)
            0xC030: LD A, 0x99         (skipped)
            0xC032: CALL NZ, 0xC050    (not taken; PC = 0xC035)
            0xC035: CALL Z, 0xC047     (taken; SP = 0xFFFC; (0xFFFD)=0xC0, (0xFFFC)=0x38; PC = 0xC047)
            0xC038: SUB 0x01           (A = 0xFF; Z=0, N=1, H=1, C=1; PC = 0xC03A)
            0xC03A: CALL NC, 0xC050    (not taken; PC = 0xC03D)
            0xC03D: CALL C, 0xC04B     (taken; SP = 0xFFFC; (0xFFFD)=0xC0, (0xFFFC)=0x40; PC = 0xC04B)
            0xC040: CALL 0xC04F        (taken; SP = 0xFFFC; (0xFFFD)=0xC0, (0xFFFC)=0x43; PC = 0xC04F)
            0xC043: JR -3              (PC = 0xC042)
            0xC045: padding
            0xC047: RET NZ             (not taken; PC = 0xC048)
            0xC048: RET C              (not taken; PC = 0xC049)
            0xC049: RET Z              (taken; SP = 0xFFFE; PC = 0xC038)
            0xC04A: padding
            0xC04B: RET Z              (not taken; PC = 0xC04C)
            0xC04C: RET NC             (not taken; PC = 0xC04D)
            0xC04D: RET C              (taken; SP = 0xFFFE; PC = 0xC040)
            0xC04E: padding
            0xC04F: RET                (taken; SP = 0xFFFE; PC = 0xC043)
        */
        load_program(&mut cpu, 0xC000, &[
            0x31, 0xFE, 0xFF, // 0xC000: LD SP, 0xFFFE
            0x01, 0xFF, 0x12, // 0xC003: LD BC, 0x12FF
            0xC5,             // 0xC006: PUSH BC
            0xF1,             // 0xC007: POP AF
            0xF5,             // 0xC008: PUSH AF
            0xD1,             // 0xC009: POP DE
            0x21, 0x12, 0xC0, // 0xC00A: LD HL, 0xC012
            0x18, 0x02,       // 0xC00D: JR +2 -> 0xC011
            0x3E, 0x99,       // 0xC00F: LD A, 0x99
            0xE9,             // 0xC011: JP (HL) -> 0xC012
            0x3E, 0x05,       // 0xC012: LD A, 0x05
            0xD6, 0x05,       // 0xC014: SUB 0x05 (Z=1, C=0)
            0x20, 0x02,       // 0xC016: JR NZ, +2
            0x28, 0x02,       // 0xC018: JR Z, +2 -> 0xC01C
            0x3E, 0x99,       // 0xC01A: LD A, 0x99
            0x38, 0x02,       // 0xC01C: JR C, +2
            0x30, 0x02,       // 0xC01E: JR NC, +2 -> 0xC022
            0x3E, 0x99,       // 0xC020: LD A, 0x99
            0xC2, 0x50, 0xC0, // 0xC022: JP NZ, 0xC050
            0xCA, 0x2A, 0xC0, // 0xC025: JP Z, 0xC02A
            0x3E, 0x99,       // 0xC028: LD A, 0x99
            0xDA, 0x50, 0xC0, // 0xC02A: JP C, 0xC050
            0xD2, 0x32, 0xC0, // 0xC02D: JP NC, 0xC032
            0x3E, 0x99,       // 0xC030: LD A, 0x99
            0xC4, 0x50, 0xC0, // 0xC032: CALL NZ, 0xC050
            0xCC, 0x47, 0xC0, // 0xC035: CALL Z, 0xC047
            0xD6, 0x01,       // 0xC038: SUB 0x01 (Z=0, C=1)
            0xD4, 0x50, 0xC0, // 0xC03A: CALL NC, 0xC050
            0xDC, 0x4B, 0xC0, // 0xC03D: CALL C, 0xC04B
            0xCD, 0x4F, 0xC0, // 0xC040: CALL 0xC04F
            0x18, 0xFD,       // 0xC043: JR -3 -> 0xC042
            0x00, 0x00,       // 0xC045..0xC046: padding
            0xC0,             // 0xC047: RET NZ
            0xD8,             // 0xC048: RET C
            0xC8,             // 0xC049: RET Z
            0x00,             // 0xC04A: padding
            0xC8,             // 0xC04B: RET Z
            0xD0,             // 0xC04C: RET NC
            0xD8,             // 0xC04D: RET C
            0x00,             // 0xC04E: padding
            0xC9,             // 0xC04F: RET
        ]);
        cpu.registers.set_pc(0xC000);

        // 16-bit immediate loads (LD rr, nn)
        assert_eq!(cpu.step(), 12); // LD SP, 0xFFFE
        assert_eq!(cpu.registers.sp(), 0xFFFE);
        assert_eq!(cpu.registers.pc, 0xC003);

        assert_eq!(cpu.step(), 12); // LD BC, 0x12FF
        assert_eq!(cpu.registers.bc(), 0x12FF);
        assert_eq!(cpu.registers.pc, 0xC006);

        // Stack operations and AF lower nibble masking (F & 0xF0)
        assert_eq!(cpu.step(), 16); // PUSH BC
        assert_eq!(cpu.registers.sp(), 0xFFFC);
        assert_eq!(cpu.bus.read(0xFFFD), 0x12);
        assert_eq!(cpu.bus.read(0xFFFC), 0xFF);
        assert_eq!(cpu.registers.pc, 0xC007);

        assert_eq!(cpu.step(), 12); // POP AF -> AF = 0x12F0
        assert_eq!(cpu.registers.a, 0x12);
        assert_eq!(cpu.registers.f, 0xF0);
        assert_eq!(cpu.registers.af(), 0x12F0);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.sp(), 0xFFFE);
        assert_eq!(cpu.registers.pc, 0xC008);

        assert_eq!(cpu.step(), 16); // PUSH AF
        assert_eq!(cpu.bus.read(0xFFFD), 0x12);
        assert_eq!(cpu.bus.read(0xFFFC), 0xF0);
        assert_eq!(cpu.registers.pc, 0xC009);

        assert_eq!(cpu.step(), 12); // POP DE -> DE = 0x12F0
        assert_eq!(cpu.registers.de(), 0x12F0);
        assert_eq!(cpu.registers.sp(), 0xFFFE);
        assert_eq!(cpu.registers.pc, 0xC00A);

        // Unconditional jumps (JR e, JP (HL))
        assert_eq!(cpu.step(), 12); // LD HL, 0xC012
        assert_eq!(cpu.registers.hl(), 0xC012);
        assert_eq!(cpu.registers.pc, 0xC00D);

        assert_eq!(cpu.step(), 12); // JR +2 -> 0xC011
        assert_eq!(cpu.registers.pc, 0xC011);

        assert_eq!(cpu.step(), 4);  // JP (HL) -> 0xC012
        assert_eq!(cpu.registers.pc, 0xC012);

        // Set flags: Z = 1, C = 0
        assert_eq!(cpu.step(), 8);  // LD A, 0x05
        assert_eq!(cpu.step(), 8);  // SUB 0x05 -> A = 0x00, Z = 1, C = 0
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), false);
        assert_eq!(cpu.registers.pc, 0xC016);

        // Conditional relative jumps (JR cc, e)
        assert_eq!(cpu.step(), 8);  // JR NZ, +2 (not taken)
        assert_eq!(cpu.registers.pc, 0xC018);

        assert_eq!(cpu.step(), 12); // JR Z, +2 (taken) -> 0xC01C
        assert_eq!(cpu.registers.pc, 0xC01C);

        assert_eq!(cpu.step(), 8);  // JR C, +2 (not taken)
        assert_eq!(cpu.registers.pc, 0xC01E);

        assert_eq!(cpu.step(), 12); // JR NC, +2 (taken) -> 0xC022
        assert_eq!(cpu.registers.pc, 0xC022);

        // Conditional absolute jumps (JP cc, a16)
        assert_eq!(cpu.step(), 12); // JP NZ, 0xC050 (not taken)
        assert_eq!(cpu.registers.pc, 0xC025);

        assert_eq!(cpu.step(), 16); // JP Z, 0xC02A (taken) -> 0xC02A
        assert_eq!(cpu.registers.pc, 0xC02A);

        assert_eq!(cpu.step(), 12); // JP C, 0xC050 (not taken)
        assert_eq!(cpu.registers.pc, 0xC02D);

        assert_eq!(cpu.step(), 16); // JP NC, 0xC032 (taken) -> 0xC032
        assert_eq!(cpu.registers.pc, 0xC032);

        // Conditional calls and returns with Z = 1, C = 0
        assert_eq!(cpu.step(), 12); // CALL NZ, 0xC050 (not taken)
        assert_eq!(cpu.registers.pc, 0xC035);

        assert_eq!(cpu.step(), 24); // CALL Z, 0xC047 (taken) -> 0xC047
        assert_eq!(cpu.registers.pc, 0xC047);
        assert_eq!(cpu.registers.sp(), 0xFFFC);
        assert_eq!(cpu.bus.read(0xFFFD), 0xC0);
        assert_eq!(cpu.bus.read(0xFFFC), 0x38);

        assert_eq!(cpu.step(), 8);  // RET NZ (not taken)
        assert_eq!(cpu.registers.pc, 0xC048);

        assert_eq!(cpu.step(), 8);  // RET C (not taken)
        assert_eq!(cpu.registers.pc, 0xC049);

        assert_eq!(cpu.step(), 20); // RET Z (taken) -> 0xC038
        assert_eq!(cpu.registers.pc, 0xC038);
        assert_eq!(cpu.registers.sp(), 0xFFFE);

        // Set flags: Z = 0, C = 1
        assert_eq!(cpu.step(), 8);  // SUB 0x01 -> A = 0xFF, Z = 0, C = 1
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC03A);

        // Conditional calls and returns with Z = 0, C = 1
        assert_eq!(cpu.step(), 12); // CALL NC, 0xC050 (not taken)
        assert_eq!(cpu.registers.pc, 0xC03D);

        assert_eq!(cpu.step(), 24); // CALL C, 0xC04B (taken) -> 0xC04B
        assert_eq!(cpu.registers.pc, 0xC04B);
        assert_eq!(cpu.registers.sp(), 0xFFFC);
        assert_eq!(cpu.bus.read(0xFFFD), 0xC0);
        assert_eq!(cpu.bus.read(0xFFFC), 0x40);

        assert_eq!(cpu.step(), 8);  // RET Z (not taken)
        assert_eq!(cpu.registers.pc, 0xC04C);

        assert_eq!(cpu.step(), 8);  // RET NC (not taken)
        assert_eq!(cpu.registers.pc, 0xC04D);

        assert_eq!(cpu.step(), 20); // RET C (taken) -> 0xC040
        assert_eq!(cpu.registers.pc, 0xC040);
        assert_eq!(cpu.registers.sp(), 0xFFFE);

        // Unconditional CALL, RET and negative relative jump
        assert_eq!(cpu.step(), 24); // CALL 0xC04F (taken) -> 0xC04F
        assert_eq!(cpu.registers.pc, 0xC04F);
        assert_eq!(cpu.registers.sp(), 0xFFFC);
        assert_eq!(cpu.bus.read(0xFFFD), 0xC0);
        assert_eq!(cpu.bus.read(0xFFFC), 0x43);

        assert_eq!(cpu.step(), 16); // RET (taken) -> 0xC043
        assert_eq!(cpu.registers.pc, 0xC043);
        assert_eq!(cpu.registers.sp(), 0xFFFE);

        assert_eq!(cpu.step(), 12); // JR -3 -> 0xC042
        assert_eq!(cpu.registers.pc, 0xC042);
    }

    #[test]
    fn test_loads_and_inc_dec_8_16() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        /*
            0xC000: LD BC, 0xC100      (BC = 0xC100; PC = 0xC003)
            0xC003: LD DE, 0xC103      (DE = 0xC103; PC = 0xC006)
            0xC006: LD HL, 0xC104      (HL = 0xC104; PC = 0xC009)
            0xC009: LD SP, 0xFFFF      (SP = 0xFFFF; PC = 0xC00C)

            // 16-bit INC/DEC (no flags modified)
            0xC00C: INC BC             (BC = 0xC101; PC = 0xC00D)
            0xC00D: DEC DE             (DE = 0xC102; PC = 0xC00E)
            0xC00E: INC SP             (SP = 0x0000; PC = 0xC00F)
            0xC00F: DEC SP             (SP = 0xFFFF; PC = 0xC010)
            0xC010: LD (0xC110), SP    ((0xC110) = 0xFF, (0xC111) = 0xFF; PC = 0xC013)

            // Indirect memory loads via (BC), (DE), (HL+), (HL-)
            0xC013: LD A, 0x42         (A = 0x42; PC = 0xC015)
            0xC015: LD (BC), A         ((0xC101) = 0x42; PC = 0xC016)
            0xC016: LD A, 0x77         (A = 0x77; PC = 0xC018)
            0xC018: LD (DE), A         ((0xC102) = 0x77; PC = 0xC019)
            0xC019: LD A, (BC)         (A = 0x42; PC = 0xC01A)
            0xC01A: LD (HL+), A        ((0xC104) = 0x42; HL = 0xC105; PC = 0xC01B)
            0xC01B: LD A, (DE)         (A = 0x77; PC = 0xC01C)
            0xC01C: LD (HL-), A        ((0xC105) = 0x77; HL = 0xC104; PC = 0xC01D)
            0xC01D: LD A, (HL+)        (A = 0x42; HL = 0xC105; PC = 0xC01E)
            0xC01E: LD A, (HL-)        (A = 0x77; HL = 0xC104; PC = 0xC01F)

            // Direct & high-RAM loads: (a16), (a8), (C)
            0xC01F: LD (0xC120), A     ((0xC120) = 0x77; PC = 0xC022)
            0xC022: LD A, 0x00         (A = 0x00; PC = 0xC024)
            0xC024: LD A, (0xC120)     (A = 0x77; PC = 0xC027)
            0xC027: LDH (0x80), A      ((0xFF80) = 0x77; PC = 0xC029)
            0xC029: LD A, 0x00         (A = 0x00; PC = 0xC02B)
            0xC02B: LDH A, (0x80)      (A = 0x77; PC = 0xC02D)
            0xC02D: LD C, 0x85         (C = 0x85; PC = 0xC02F)
            0xC02F: LD (C), A          ((0xFF85) = 0x77; PC = 0xC030)
            0xC030: LD A, 0x00         (A = 0x00; PC = 0xC032)
            0xC032: LD A, (C)          (A = 0x77; PC = 0xC033)

            // 8-bit immediate loads & INC/DEC (register & (HL), C flag preserved)
            0xC033: LD B, 0x0F         (B = 0x0F; PC = 0xC035)
            0xC035: INC B              (B = 0x10; Z=0, N=0, H=1, C=1; PC = 0xC036)
            0xC036: DEC B              (B = 0x0F; Z=0, N=1, H=1, C=1; PC = 0xC037)
            0xC037: LD C, 0xFF         (C = 0xFF; PC = 0xC039)
            0xC039: INC C              (C = 0x00; Z=1, N=0, H=1, C=1; PC = 0xC03A)
            0xC03A: DEC C              (C = 0xFF; Z=0, N=1, H=1, C=1; PC = 0xC03B)
            0xC03B: LD (HL), 0x00      ((0xC104) = 0x00; PC = 0xC03D)
            0xC03D: DEC (HL)           ((0xC104) = 0xFF; Z=0, N=1, H=1, C=1; PC = 0xC03E)
            0xC03E: INC (HL)           ((0xC104) = 0x00; Z=1, N=0, H=1, C=1; PC = 0xC03F)
        */
        load_program(&mut cpu, 0xC000, &[
            0x01, 0x00, 0xC1, // 0xC000: LD BC, 0xC100
            0x11, 0x03, 0xC1, // 0xC003: LD DE, 0xC103
            0x21, 0x04, 0xC1, // 0xC006: LD HL, 0xC104
            0x31, 0xFF, 0xFF, // 0xC009: LD SP, 0xFFFF
            0x03,             // 0xC00C: INC BC
            0x1B,             // 0xC00D: DEC DE
            0x33,             // 0xC00E: INC SP
            0x3B,             // 0xC00F: DEC SP
            0x08, 0x10, 0xC1, // 0xC010: LD (0xC110), SP
            0x3E, 0x42,       // 0xC013: LD A, 0x42
            0x02,             // 0xC015: LD (BC), A
            0x3E, 0x77,       // 0xC016: LD A, 0x77
            0x12,             // 0xC018: LD (DE), A
            0x0A,             // 0xC019: LD A, (BC)
            0x22,             // 0xC01A: LD (HL+), A
            0x1A,             // 0xC01B: LD A, (DE)
            0x32,             // 0xC01C: LD (HL-), A
            0x2A,             // 0xC01D: LD A, (HL+)
            0x3A,             // 0xC01E: LD A, (HL-)
            0xEA, 0x20, 0xC1, // 0xC01F: LD (0xC120), A
            0x3E, 0x00,       // 0xC022: LD A, 0x00
            0xFA, 0x20, 0xC1, // 0xC024: LD A, (0xC120)
            0xE0, 0x80,       // 0xC027: LDH (0x80), A
            0x3E, 0x00,       // 0xC029: LD A, 0x00
            0xF0, 0x80,       // 0xC02B: LDH A, (0x80)
            0x0E, 0x85,       // 0xC02D: LD C, 0x85
            0xE2,             // 0xC02F: LD (C), A
            0x3E, 0x00,       // 0xC030: LD A, 0x00
            0xF2,             // 0xC032: LD A, (C)
            0x06, 0x0F,       // 0xC033: LD B, 0x0F
            0x04,             // 0xC035: INC B
            0x05,             // 0xC036: DEC B
            0x0E, 0xFF,       // 0xC037: LD C, 0xFF
            0x0C,             // 0xC039: INC C
            0x0D,             // 0xC03A: DEC C
            0x36, 0x00,       // 0xC03B: LD (HL), 0x00
            0x35,             // 0xC03D: DEC (HL)
            0x34,             // 0xC03E: INC (HL)
        ]);
        cpu.registers.set_pc(0xC000);
        cpu.registers.set_flag(registers::FLAG_Z, false);
        cpu.registers.set_flag(registers::FLAG_N, false);
        cpu.registers.set_flag(registers::FLAG_H, false);
        cpu.registers.set_flag(registers::FLAG_C, true);

        // 16-bit immediate loads (LD rr, nn)
        assert_eq!(cpu.step(), 12); // LD BC, 0xC100
        assert_eq!(cpu.registers.bc(), 0xC100);
        assert_eq!(cpu.registers.pc, 0xC003);

        assert_eq!(cpu.step(), 12); // LD DE, 0xC103
        assert_eq!(cpu.registers.de(), 0xC103);
        assert_eq!(cpu.registers.pc, 0xC006);

        assert_eq!(cpu.step(), 12); // LD HL, 0xC104
        assert_eq!(cpu.registers.hl(), 0xC104);
        assert_eq!(cpu.registers.pc, 0xC009);

        assert_eq!(cpu.step(), 12); // LD SP, 0xFFFF
        assert_eq!(cpu.registers.sp(), 0xFFFF);
        assert_eq!(cpu.registers.pc, 0xC00C);

        // 16-bit INC/DEC & SP store (no flags modified)
        assert_eq!(cpu.step(), 8);  // INC BC -> 0xC101
        assert_eq!(cpu.registers.bc(), 0xC101);
        assert_eq!(cpu.registers.pc, 0xC00D);

        assert_eq!(cpu.step(), 8);  // DEC DE -> 0xC102
        assert_eq!(cpu.registers.de(), 0xC102);
        assert_eq!(cpu.registers.pc, 0xC00E);

        assert_eq!(cpu.step(), 8);  // INC SP -> 0x0000
        assert_eq!(cpu.registers.sp(), 0x0000);
        assert_eq!(cpu.registers.pc, 0xC00F);

        assert_eq!(cpu.step(), 8);  // DEC SP -> 0xFFFF
        assert_eq!(cpu.registers.sp(), 0xFFFF);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC010);

        assert_eq!(cpu.step(), 20); // LD (0xC110), SP -> (0xC110)=0xFF, (0xC111)=0xFF
        assert_eq!(cpu.bus.read(0xC110), 0xFF);
        assert_eq!(cpu.bus.read(0xC111), 0xFF);
        assert_eq!(cpu.registers.pc, 0xC013);

        // Indirect memory loads via (BC), (DE), (HL+), (HL-)
        assert_eq!(cpu.step(), 8);  // LD A, 0x42
        assert_eq!(cpu.step(), 8);  // LD (BC), A -> (0xC101) = 0x42
        assert_eq!(cpu.bus.read(0xC101), 0x42);
        assert_eq!(cpu.registers.pc, 0xC016);

        assert_eq!(cpu.step(), 8);  // LD A, 0x77
        assert_eq!(cpu.step(), 8);  // LD (DE), A -> (0xC102) = 0x77
        assert_eq!(cpu.bus.read(0xC102), 0x77);
        assert_eq!(cpu.registers.pc, 0xC019);

        assert_eq!(cpu.step(), 8);  // LD A, (BC) -> A = 0x42
        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.registers.pc, 0xC01A);

        assert_eq!(cpu.step(), 8);  // LD (HL+), A -> (0xC104) = 0x42, HL = 0xC105
        assert_eq!(cpu.bus.read(0xC104), 0x42);
        assert_eq!(cpu.registers.hl(), 0xC105);
        assert_eq!(cpu.registers.pc, 0xC01B);

        assert_eq!(cpu.step(), 8);  // LD A, (DE) -> A = 0x77
        assert_eq!(cpu.registers.a, 0x77);
        assert_eq!(cpu.registers.pc, 0xC01C);

        assert_eq!(cpu.step(), 8);  // LD (HL-), A -> (0xC105) = 0x77, HL = 0xC104
        assert_eq!(cpu.bus.read(0xC105), 0x77);
        assert_eq!(cpu.registers.hl(), 0xC104);
        assert_eq!(cpu.registers.pc, 0xC01D);

        assert_eq!(cpu.step(), 8);  // LD A, (HL+) -> A = 0x42, HL = 0xC105
        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.registers.hl(), 0xC105);
        assert_eq!(cpu.registers.pc, 0xC01E);

        assert_eq!(cpu.step(), 8);  // LD A, (HL-) -> A = 0x77, HL = 0xC104
        assert_eq!(cpu.registers.a, 0x77);
        assert_eq!(cpu.registers.hl(), 0xC104);
        assert_eq!(cpu.registers.pc, 0xC01F);

        // Direct 16-bit & High-RAM loads: (a16), (a8), (C)
        assert_eq!(cpu.step(), 16); // LD (0xC120), A -> (0xC120) = 0x77
        assert_eq!(cpu.bus.read(0xC120), 0x77);
        assert_eq!(cpu.registers.pc, 0xC022);

        assert_eq!(cpu.step(), 8);  // LD A, 0x00
        assert_eq!(cpu.step(), 16); // LD A, (0xC120) -> A = 0x77
        assert_eq!(cpu.registers.a, 0x77);
        assert_eq!(cpu.registers.pc, 0xC027);

        assert_eq!(cpu.step(), 12); // LDH (0x80), A -> (0xFF80) = 0x77
        assert_eq!(cpu.bus.read(0xFF80), 0x77);
        assert_eq!(cpu.registers.pc, 0xC029);

        assert_eq!(cpu.step(), 8);  // LD A, 0x00
        assert_eq!(cpu.step(), 12); // LDH A, (0x80) -> A = 0x77
        assert_eq!(cpu.registers.a, 0x77);
        assert_eq!(cpu.registers.pc, 0xC02D);

        assert_eq!(cpu.step(), 8);  // LD C, 0x85
        assert_eq!(cpu.step(), 8);  // LD (C), A -> (0xFF85) = 0x77
        assert_eq!(cpu.bus.read(0xFF85), 0x77);
        assert_eq!(cpu.registers.pc, 0xC030);

        assert_eq!(cpu.step(), 8);  // LD A, 0x00
        assert_eq!(cpu.step(), 8);  // LD A, (C) -> A = 0x77
        assert_eq!(cpu.registers.a, 0x77);
        assert_eq!(cpu.registers.pc, 0xC033);

        // 8-bit immediate loads & INC/DEC on registers (C flag must remain true)
        assert_eq!(cpu.step(), 8);  // LD B, 0x0F
        assert_eq!(cpu.step(), 4);  // INC B -> 0x10 (Z=0, N=0, H=1, C=1)
        assert_eq!(cpu.registers.b, 0x10);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC036);

        assert_eq!(cpu.step(), 4);  // DEC B -> 0x0F (Z=0, N=1, H=1, C=1)
        assert_eq!(cpu.registers.b, 0x0F);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC037);

        assert_eq!(cpu.step(), 8);  // LD C, 0xFF
        assert_eq!(cpu.step(), 4);  // INC C -> 0x00 (Z=1, N=0, H=1, C=1)
        assert_eq!(cpu.registers.c, 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC03A);

        assert_eq!(cpu.step(), 4);  // DEC C -> 0xFF (Z=0, N=1, H=1, C=1)
        assert_eq!(cpu.registers.c, 0xFF);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC03B);

        // 8-bit immediate load & INC/DEC on (HL) (12 cycles each)
        assert_eq!(cpu.step(), 12); // LD (HL), 0x00 -> (0xC104) = 0x00
        assert_eq!(cpu.bus.read(0xC104), 0x00);
        assert_eq!(cpu.registers.pc, 0xC03D);

        assert_eq!(cpu.step(), 12); // DEC (HL) -> (0xC104) = 0xFF (Z=0, N=1, H=1, C=1)
        assert_eq!(cpu.bus.read(0xC104), 0xFF);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC03E);

        assert_eq!(cpu.step(), 12); // INC (HL) -> (0xC104) = 0x00 (Z=1, N=0, H=1, C=1)
        assert_eq!(cpu.bus.read(0xC104), 0x00);
        assert_eq!(cpu.registers.flag(registers::FLAG_Z), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_N), false);
        assert_eq!(cpu.registers.flag(registers::FLAG_H), true);
        assert_eq!(cpu.registers.flag(registers::FLAG_C), true);
        assert_eq!(cpu.registers.pc, 0xC03F);
    }

}