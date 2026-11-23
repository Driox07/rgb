pub const TIMER_IF_BIT: u8 = 2;

pub const TMA_0: u8 = 0;
pub const TMA_1: u8 = 1;
pub const TMA_2: u8 = 2;
pub const TMA_3: u8 = 3;

const TMA_MODE_MASKS: [u16; 4] = [0x0200, 0x0008, 0x0020, 0x0080];

pub struct Timer {
    system_counter: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    last_state: bool,
    tima_load_pending: bool,
    tima_cycles_left: u8
}

impl Timer {

    pub fn new() -> Timer {

        Timer {
            system_counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            last_state: false,
            tima_load_pending: false,
            tima_cycles_left: 3
        }

    }

    /* 
    TAC
    7 6 5 4 3 2 1 0
    --------- - ---
    No use    E  F

    E -> Enable: 1=enabled 0=disabled
    F -> Freq. : 00=1024, 01=16, 10=64, 11=256
                   (b9),    (b3),  (b5),  (b7) 
    */


    pub fn div(&self) -> u8 {
        (self.system_counter >> 8) as u8
    }

    pub fn set_div(&mut self) {
        self.system_counter = 0;
        self.update_tima();
    }

    pub fn tima(&self) -> u8 {
        self.tima
    }

    pub fn set_tima(&mut self, value: u8) {
        if self.tima_load_pending {
            self.tima_load_pending = false;
            self.tima_cycles_left = 3;
        }
        self.tima = value;
    }

    pub fn tma(&self) -> u8 {
        self.tma
    }

    pub fn set_tma(&mut self, value: u8) {
        self.tma = value;
    }

    pub fn tac(&self) -> u8 {
        self.tac | 0xF8
    }

    pub fn set_tac(&mut self, value: u8) {
        self.tac = value & 7;
        self.update_tima();
    }

    fn load_tima(&mut self) -> bool {
        if self.tima_load_pending{
            if self.tima_cycles_left == 0{
                self.tima_load_pending = false;
                self.tima_cycles_left = 3;
                self.tima = self.tma;
                true
            }else{
                self.tima_cycles_left -= 1;
                false
            }
        }else{
            false
        }
    }

    fn update_tima(&mut self){
        if self.tac & 4 != 0 { // TIMA ON
            let tima_mode = self.tac & 0x3;
            let curr_state = self.system_counter & TMA_MODE_MASKS[tima_mode as usize] != 0;
            if self.last_state && !curr_state {
                self.tima_load_pending |= self.tima == 0xFF;
                self.tima = self.tima.wrapping_add(1);
            }
            self.last_state = curr_state;
        }else{
            if self.last_state {
                self.tima_load_pending |= self.tima == 0xFF;
                self.tima = self.tima.wrapping_add(1);
                self.last_state = false;
            }
        }
    }

    pub fn tick(&mut self, t: u8) -> bool {
        let mut timer_int = false;
        for _ in 0..t {
            timer_int |= self.load_tima();
            self.system_counter = self.system_counter.wrapping_add(1);
            self.update_tima();
        }
        timer_int
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_increment_and_reset() {
        let mut timer = Timer::new();

        timer.tick(255);
        assert_eq!(timer.div(), 0);

        timer.tick(1);
        assert_eq!(timer.div(), 1);

        timer.tick(255);
        timer.tick(1);
        assert_eq!(timer.div(), 2);

        // Writing to DIV resets the internal 16-bit system counter to 0
        timer.set_div();
        assert_eq!(timer.div(), 0);
    }

    #[test]
    fn test_tac_unused_bits() {
        let mut timer = Timer::new();
        timer.set_tac(0b0000_0101);
        // Bits 7-3 must always read as 1
        assert_eq!(timer.tac(), 0b1111_1101);
    }

    #[test]
    fn test_tima_disabled() {
        let mut timer = Timer::new();
        timer.set_tac(0b000);
        timer.tick(100);
        assert_eq!(timer.tima(), 0);
    }

    #[test]
    fn test_tima_frequencies() {
        // (TAC value, T-cycles per TIMA increment)
        let cases: [(u8, usize); 4] = [
            (0b100, 1024), // Mode 00: 4096 Hz
            (0b101, 16),   // Mode 01: 262144 Hz
            (0b110, 64),   // Mode 10: 65536 Hz
            (0b111, 256),  // Mode 11: 16384 Hz
        ];

        for (tac_val, period) in cases {
            let mut timer = Timer::new();
            timer.set_tac(tac_val);

            // Advance to one T-cycle before the expected falling edge
            let mut remaining = period - 1;
            while remaining > 0 {
                let step = remaining.min(16) as u8;
                timer.tick(step);
                remaining -= step as usize;
            }
            assert_eq!(timer.tima(), 0);

            // Complete the period to trigger the falling edge
            timer.tick(1);
            assert_eq!(timer.tima(), 1);
        }
    }

    #[test]
    fn test_tima_overflow_delay_and_reload() {
        let mut timer = Timer::new();
        timer.set_tma(0xAB);
        timer.set_tima(0xFF);
        timer.set_tac(0b101);

        // TIMA overflows to 0x00; reload and interrupt are delayed by 4 T-cycles
        assert!(!timer.tick(16));
        assert_eq!(timer.tima(), 0x00);

        // Cycles 1 to 3 after overflow: TIMA remains 0x00, no interrupt requested
        assert!(!timer.tick(3));
        assert_eq!(timer.tima(), 0x00);

        // Cycle 4 after overflow: TIMA reloads from TMA and interrupt is requested
        assert!(timer.tick(1));
        assert_eq!(timer.tima(), 0xAB);
    }

    #[test]
    fn test_write_tima_during_overflow_delay() {
        let mut timer = Timer::new();
        timer.set_tma(0xAB);
        timer.set_tima(0xFF);
        timer.set_tac(0b101);

        // Trigger TIMA overflow
        assert!(!timer.tick(16));
        assert_eq!(timer.tima(), 0x00);

        // Writing to TIMA during the 4-cycle delay cancels TMA reload and interrupt
        assert!(!timer.tick(2));
        timer.set_tima(0x42);

        assert!(!timer.tick(4));
        assert_eq!(timer.tima(), 0x42);
    }

    #[test]
    fn test_falling_edge_on_div_reset() {
        let mut timer = Timer::new();
        timer.set_tac(0b101);

        // Set watched bit (bit 3) high
        timer.tick(8);
        assert_eq!(timer.tima(), 0);

        // Resetting DIV forces watched bit low, triggering a falling edge
        timer.set_div();
        assert_eq!(timer.tima(), 1);
    }

    #[test]
    fn test_falling_edge_on_tac_disable() {
        let mut timer = Timer::new();
        timer.set_tac(0b101);

        // Set watched bit (bit 3) high
        timer.tick(8);
        assert_eq!(timer.tima(), 0);

        // Disabling TAC forces internal signal low, triggering a falling edge
        timer.set_tac(0b001);
        assert_eq!(timer.tima(), 1);

        // Re-enabling TAC while watched bit is low must not trigger an increment
        timer.tick(8);
        timer.set_tac(0b101);
        assert_eq!(timer.tima(), 1);
    }
}