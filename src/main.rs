use eframe::egui;
use rgb::gbcore;
use rgb::gbcore::CPU;
use rgb::gbcore::Bus;

use std::path::PathBuf;

const FRAME_SECS: f64 = 70224.0 / 4194304.0;
const SPEED_MULT: f64 = 15.0;

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
    palette: [egui::Color32; 4],
    last_frame: std::time::Instant,
    elapsed: f64,
    save_path: Option<PathBuf>
}

impl EmulatorApp {
    fn new(cc: &eframe::CreationContext<'_>, rom_path: String) -> EmulatorApp {
        let mut emulator = EmulatorApp {
            cpu: CPU::new(Bus::new()),
            texture: cc.egui_ctx.load_texture("screen",
               egui::ColorImage::new([160, 144], vec![egui::Color32::BLACK; 160*144]), 
             egui::TextureOptions::NEAREST),

            palette: [egui::Color32::from_rgb(0x9B, 0xBC, 0x0F),
                      egui::Color32::from_rgb(0x8B, 0xAC, 0x0F),
                      egui::Color32::from_rgb(0x30, 0x62, 0x30),
                      egui::Color32::from_rgb(0x0F, 0x38, 0x0F),
                     ],
            last_frame: std::time::Instant::now(),
            elapsed: 0.0,
            save_path: None
        };

        let rom = match std::fs::read( &rom_path ){
            Ok(vector) => vector,
            Err(error) => {
                panic!("{}", error)
            }
        };

        let save_path = Some(std::path::Path::new(&rom_path).with_extension("sav"));
        emulator.save_path = save_path;

        match emulator.cpu.bus.load_rom(rom) {
            Ok(()) => (),
            Err(error) => {
                panic!("{}", error)
            }
        };

        emulator.load_save();

        emulator
    }

    fn run_frame(&mut self){
        loop {
            let t = self.cpu.step();

            self.cpu.bus.tick_timer(t);

            let vblank = self.cpu.bus.tick_ppu(t);

            let serial = self.cpu.bus.fetch_serial();

            self.cpu.bus.tick_cartridge(t);

            if serial != None {
                print!("{}", serial.unwrap() as char);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            
            if vblank { break; }
        }
    }

    fn save_game(&mut self) {
        if self.cpu.bus.cartridge_ram_written() {
            match self.cpu.bus.cartridge_ram() {
                Err(error) => print!("Saving game failed: {}", error),
                Ok(ram) => {

                    match &self.save_path {
                    None => println!("No save path specified."),
                    Some(path) => match std::fs::write(path, ram) {
                                            Err(error) => println!("Saving game failed: {}", error),
                                            Ok(()) => println!("Game saved")
                        }
                    }
                }
            };
        }
    }

    pub fn load_save(&mut self) {
        let save_path = match &self.save_path {
            None => return,
            Some(path) => path
        };
        let save_data = match std::fs::read(save_path) {
            Ok(data) => data,
            Err(error) => {
                println!("Save data load failed: {}", error);
                return;
            }
        };
        match self.cpu.bus.catridge_set_ram(save_data.as_slice()) {
            Ok(()) => (),
            Err(error) => println!("Save data load failed: {}", error)
        }
    }

}

impl eframe::App for EmulatorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let space = ui.input(|i| i.key_down(egui::Key::Space));
            let up = ui.input(|i| i.key_down(egui::Key::ArrowUp));
            let select = ui.input(|i| i.key_down(egui::Key::ShiftRight));
            let down = ui.input(|i| i.key_down(egui::Key::ArrowDown));
            let start = ui.input(|i| i.key_down(egui::Key::Enter));
            let left = ui.input(|i| i.key_down(egui::Key::ArrowLeft));
            let b = ui.input(|i| i.key_down(egui::Key::Z));
            let right = ui.input(|i| i.key_down(egui::Key::ArrowRight));
            let a = ui.input(|i| i.key_down(egui::Key::X));

            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_UP, up);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_SELECT, select);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_DOWN, down);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_START, start);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_LEFT, left);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_B, b);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_RIGHT, right);
            self.cpu.bus.set_joypad_key(gbcore::joypad::KEY_A, a);

            if a || b || select || start {
                println!("A:{} B:{} Select:{} Start:{}", a, b, select, start);
            }

            let now = std::time::Instant::now();
            self.elapsed += (now - self.last_frame).as_secs_f64();
            self.last_frame = now;
            self.elapsed = self.elapsed.min(FRAME_SECS * 3.0);

            let wait_secs = FRAME_SECS / if space { SPEED_MULT } else { 1.0 };

            // Draw code
            while self.elapsed >= wait_secs {
                self.run_frame();
                self.elapsed -= wait_secs;
            }

            self.save_game();

            let pixels: Vec<egui::Color32> = self.cpu.bus.ppu_framebuffer()
                                            .iter()
                                            .map(|&tono| self.palette[tono as usize])
                                            .collect();
            let image = egui::ColorImage::new([160, 144], pixels);
            self.texture.set(image, egui::TextureOptions::NEAREST);   // 3. subir
            egui::CentralPanel::default().show(ui, |ui| {           // 4. dibujar
                    ui.add(egui::Image::from_texture(&self.texture)
                    .fit_to_exact_size(egui::vec2(640.0, 576.0)));
            });
            ui.ctx().request_repaint();
        });
    }

}