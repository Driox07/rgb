use rgb::gbcore::Bus;
use rgb::gbcore::CPU;

fn main() {
    let mut bus = Bus::new();
    let path = std::env::args().nth(1);
    match path {
        None => panic!("ROM Path not specified!"),
        _=> ()
    }
    let rom = match std::fs::read( path.unwrap() ){
        Ok(vector) => vector,
        Err(error) => {
            println!("{}", error);
            return;
        }
    };
    match bus.load_rom(rom) {
        Ok(()) => (),
        Err(error) => {
            println!("{}", error);
            return;
        }
    };
    let mut cpu = CPU::new(bus);
    loop { 
        let pc = cpu.registers.pc();
        cpu.step();
        let serial = cpu.bus.fetch_serial();
        if serial != None {
            print!("{}", serial.unwrap() as char);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        if cpu.registers.pc() == pc {
            println!();
            println!("ROM entered an infinite loop at PC {:#06X}", pc);
            break;
        }
    };
    return
}
