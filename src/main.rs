use eframe::egui;
use rgb::gbcore::CPU;
use rgb::gbcore::Bus;

fn main() {

    let path = std::env::args().nth(1);
    match path {
        None => panic!("ROM Path not specified!"),
        _=> ()
    }

    eframe::run_native(
        "rgb", 
        eframe::NativeOptions::default(), 
        Box::new(move |cc| Ok(Box::new(EmulatorApp::new(cc, path.unwrap())))),
    ).unwrap();

}

struct EmulatorApp {
    cpu: CPU,
    texture: egui::TextureHandle,
    palette: [egui::Color32; 4]
}

impl EmulatorApp {
    fn new(cc: &eframe::CreationContext<'_>, rom_path: String) -> EmulatorApp {
        let mut emulator = EmulatorApp {
            cpu: CPU::new(Bus::new()),

            texture: cc.egui_ctx.load_texture("screen",
               egui::ColorImage::new([160, 144], vec![egui::Color32::BLACK; 160*144]), 
             egui::TextureOptions::NEAREST),

            palette: [egui::Color32::from_rgb(0x9B, 0xBC, 0x0F), // 0: más claro
                      egui::Color32::from_rgb(0x8B, 0xAC, 0x0F), // 1
                      egui::Color32::from_rgb(0x30, 0x62, 0x30), // 2
                      egui::Color32::from_rgb(0x0F, 0x38, 0x0F), // 3: más oscuro
                     ],
        };

        let rom = match std::fs::read( rom_path ){
            Ok(vector) => vector,
            Err(error) => {
                panic!("{}", error)
            }
        };

        match emulator.cpu.bus.load_rom(rom) {
            Ok(()) => (),
            Err(error) => {
                panic!("{}", error)
            }
        };

        emulator
    }

    fn run_frame(&mut self){
        loop {
            let t = self.cpu.step();

            self.cpu.bus.tick_timer(t);

            let vblank = self.cpu.bus.tick_ppu(t);

            let serial = self.cpu.bus.fetch_serial();

            if serial != None {
                print!("{}", serial.unwrap() as char);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            
            if vblank { break; }
        }
    }

}

impl eframe::App for EmulatorApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        
    }

}