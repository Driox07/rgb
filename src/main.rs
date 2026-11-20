use rgb::gbcore::Bus;

fn show_menu() {
    println!("What you wanna do?");
    println!("1. Test CPU");
    println!("");
    println!("0. Exit");
    println!("");
}

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
