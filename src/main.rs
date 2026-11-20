use rgb::gbcore::Bus;

fn main() {
    let mut bus = Bus::new();
    let rom = match std::fs::read("./gb-test-roms/halt_bug.gb"){
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
    return;
}
