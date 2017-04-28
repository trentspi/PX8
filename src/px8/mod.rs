pub mod info;
pub mod cartdata;
pub mod emscripten;

use std::collections::HashMap;
use std::io::BufReader;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use time;

use chan::{Receiver, Sender};

use nalgebra::clamp;

use image;

use gif;
use gif::SetParameter;

use std::io::prelude::*;

use std::path::Path;
use std::fs::File;

use plugins::lua_plugin::plugin::LuaPlugin;
use plugins::python_plugin::plugin::PythonPlugin;

use config::Players;
use self::info::Info;
use gfx;
use cartridge::{Cartridge, CartridgeFormat};
use sound::sound::Sound;

include!(concat!(env!("OUT_DIR"), "/parameters.rs"));

pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;
pub const SCREEN_PIXELS_RGB: usize = SCREEN_PIXELS * 3;

pub type ScreenBuffer = [u32; SCREEN_PIXELS];
pub type ScreenBufferRGB = [u8; SCREEN_PIXELS_RGB];

pub const SCREEN_EMPTY: ScreenBuffer = [0; SCREEN_PIXELS];

pub struct Palette {
    colors: HashMap<u32, RGB>,
    rcolors: HashMap<u32, u32>,
    cached_colors: [u32; 16],
    idx: u32,
}

impl Palette {
    pub fn new() -> Palette {
        Palette {
            colors: HashMap::new(),
            rcolors: HashMap::new(),
            cached_colors: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            idx: 16,
        }
    }

    pub fn get_rgb(&mut self, value: u32) -> RGB {
        if value < 16 {
            let v = self.cached_colors[value as usize];

            let r = ((v & 0xff0000) >> 16) as u8;
            let g = ((v & 0x00ff00) >> 8) as u8;
            let b = (v & 0x0000ff) as u8;

            return RGB::new(r, g, b);
        }

        match self.colors.get(&value) {
            Some(rgb_value) => RGB::new(rgb_value.r, rgb_value.g, rgb_value.b),
            _ => RGB::new(0, 0, 0),
        }
    }

    pub fn reset(&mut self) {
        self.colors.clear();
    }

    pub fn set_color(&mut self, color: u32, r: u8, g: u8, b: u8) {
        let u32_color = (r as u32) << 16 | (g as u32) << 8 | (b as u32);

        self.colors.insert(color, RGB::new(r, g, b));
        self.rcolors.insert(u32_color, color);
        if color < 16 {
            self.cached_colors[color as usize] = u32_color;
        }
    }

    pub fn get_color(&mut self, color: u32) -> u32 {
        match self.colors.get(&color) {
            Some(rgb_value) => return (rgb_value.r as u32) << 16 | (rgb_value.g as u32) << 8 | (rgb_value.b as u32),
            _ => return 0,
        }
    }

    pub fn add_color(&mut self, r: u8, g: u8, b: u8) -> u32 {
        let value = self.idx;

        let v = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
        match self.rcolors.get(&v) {
            Some(color) => return *color,
            _ => (),
        }

        self.set_color(value, r, g, b);
        self.idx += 1;

        value
    }
}

lazy_static! {
    pub static ref PALETTE: Mutex<Palette> = {
        let m = Mutex::new(Palette::new());
        m
    };
}

#[derive(Clone)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> RGB {
        RGB {
            r: r,
            g: g,
            b: b
        }
    }

    pub fn new_hexa(v: u32) -> RGB {
        RGB {
            r: ((v & 0xff0000) >> 16) as u8,
            g: ((v & 0x00ff00) >> 8) as u8,
            b: (v & 0x0000ff) as u8,
        }
    }
}

pub trait RustPlugin {
    fn init(&mut self, screen: Arc<Mutex<gfx::Screen>>) -> f64;
    fn update(&mut self, players: Arc<Mutex<Players>>) -> f64;
    fn draw(&mut self, screen: Arc<Mutex<gfx::Screen>>) -> f64;
}

#[derive(PartialEq)]
pub enum PX8Mode {
    PX8,
    PICO8,
}

pub enum PX8State {
    RUN,
    PAUSE,
}

pub enum Code {
    UNKNOWN = 0,
    LUA = 1,
    PYTHON = 2,
    RUST = 3,
}

pub struct Menu {
    idx: u32,
    selected_idx: i32,
    items: Vec<String>,
}

impl Menu {
    pub fn new() -> Menu {
        let mut items = Vec::new();

        items.push("Continue".to_string());
        items.push("Config".to_string());
        items.push("Quit".to_string());

        Menu {
            idx: 0,
            selected_idx: -1,
            items: items.clone(),
        }
    }

    pub fn reset(&mut self) {
        self.selected_idx = -1;
        self.idx = 0;
    }

    pub fn stop(&mut self) -> bool {
        // Continue is clicked
        self.selected_idx == 0
    }

    pub fn update(&mut self, players: Arc<Mutex<Players>>) -> bool {
        if players.lock().unwrap().btnp(0, 6) {
            self.selected_idx = self.idx as i32;
            if self.selected_idx == self.items.len() as i32 {
                return false;
            }
        }
        else {
            if players.lock().unwrap().btnp(0, 2) {
                self.idx = clamp(self.idx - 1, 0, (self.items.len() as u32) - 1);
            }

            if players.lock().unwrap().btnp(0, 3) {
                self.idx = clamp(self.idx + 1, 0, (self.items.len() as u32) - 1);
            }
        }

        return true;
    }

    pub fn draw(&mut self, screen: Arc<Mutex<gfx::Screen>>) {
        if self.selected_idx == -1 {
            let idx_x = (SCREEN_WIDTH / 2 - 20) as i32;
            let idx_y = (SCREEN_WIDTH / 2 - 10) as i32;

            screen.lock().unwrap().rectfill(idx_x, idx_y,
                                            idx_x + 10 * self.items.len() as i32,
                                            idx_y + 15 * self.items.len() as i32,
                                            0);

            screen.lock().unwrap().pset(idx_x, idx_y + (self.idx as i32) * 10, 7);

            let mut pos = 0;
            for item in &self.items {
                screen.lock().unwrap().print(item.to_string(), idx_x + 5, idx_y + pos * 10, 7);
                pos += 1;
            }
        }

        if self.selected_idx == 1 {
            screen.lock().unwrap().cls();
            // screen.lock().unwrap().print(item.to_string(), 50, 55 + pos * 10, Color::White);
        }

    }
}

pub struct Record {
    pub recording: bool,
    pub images: Vec<u8>,
    pub filename: String,
    pub nb: i32,
}

impl Record {
    pub fn new() -> Record {
        let images = Vec::new();

        Record {
            recording: false,
            images: images,
            filename: "".to_string(),
            nb: 0,
        }
    }
}

pub struct Palettes {
    pub palette_idx: u32,
    pub palettes: HashMap<String, Vec<RGB>>,
    pub palettes_list: Vec<String>,
    pub name: String,
}

impl Palettes {
    pub fn new() -> Palettes {
        Palettes {
            palette_idx: 0,
            palettes: HashMap::new(),
            palettes_list: Vec::new(),
            name: "".to_string(),
        }
    }

    pub fn init(&mut self) {
        // load palettes statically for emscripten
        self.load("a64".to_string(), include_str!("../../sys/assets/palettes/a64.gpl").to_string());
        self.load("apple-ii".to_string(), include_str!("../../sys/assets/palettes/apple-ii.gpl").to_string());
        self.load("arne-paldac".to_string(), include_str!("../../sys/assets/palettes/arne-paldac.gpl").to_string());
        self.load("arne16".to_string(), include_str!("../../sys/assets/palettes/arne16.gpl").to_string());
        self.load("arne32".to_string(), include_str!("../../sys/assets/palettes/arne32.gpl").to_string());
        self.load("atari2600-ntsc".to_string(), include_str!("../../sys/assets/palettes/atari2600-ntsc.gpl").to_string());
        self.load("atari2600-pal".to_string(), include_str!("../../sys/assets/palettes/atari2600-pal.gpl").to_string());
        self.load("cg-arne".to_string(), include_str!("../../sys/assets/palettes/cg-arne.gpl").to_string());
        self.load("cga".to_string(), include_str!("../../sys/assets/palettes/cga.gpl").to_string());
        self.load("commodore-plus4".to_string(), include_str!("../../sys/assets/palettes/commodore-plus4.gpl").to_string());
        self.load("commodore-vic20".to_string(), include_str!("../../sys/assets/palettes/commodore-vic20.gpl").to_string());
        self.load("commodore64".to_string(), include_str!("../../sys/assets/palettes/commodore64.gpl").to_string());
        self.load("copper-tech".to_string(), include_str!("../../sys/assets/palettes/copper-tech.gpl").to_string());
        self.load("cpc-boy".to_string(), include_str!("../../sys/assets/palettes/cpc-boy.gpl").to_string());
        self.load("db16".to_string(), include_str!("../../sys/assets/palettes/db16.gpl").to_string());
        self.load("db32".to_string(), include_str!("../../sys/assets/palettes/db32.gpl").to_string());
        self.load("edg16".to_string(), include_str!("../../sys/assets/palettes/edg16.gpl").to_string());
        self.load("edg32".to_string(), include_str!("../../sys/assets/palettes/edg32.gpl").to_string());
        self.load("eroge-copper".to_string(), include_str!("../../sys/assets/palettes/eroge-copper.gpl").to_string());
        self.load("gameboy-color-type1".to_string(), include_str!("../../sys/assets/palettes/gameboy-color-type1.gpl").to_string());
        self.load("gameboy".to_string(), include_str!("../../sys/assets/palettes/gameboy.gpl").to_string());
        self.load("google-ui".to_string(), include_str!("../../sys/assets/palettes/google-ui.gpl").to_string());
        self.load("jmp".to_string(), include_str!("../../sys/assets/palettes/jmp.gpl").to_string());
        self.load("mail24".to_string(), include_str!("../../sys/assets/palettes/mail24.gpl").to_string());
        self.load("master-system".to_string(), include_str!("../../sys/assets/palettes/master-system.gpl").to_string());
        self.load("monokai".to_string(), include_str!("../../sys/assets/palettes/monokai.gpl").to_string());
        self.load("nes-ntsc".to_string(), include_str!("../../sys/assets/palettes/nes-ntsc.gpl").to_string());
        self.load("nes".to_string(), include_str!("../../sys/assets/palettes/nes.gpl").to_string());
        self.load("pico-8".to_string(), include_str!("../../sys/assets/palettes/pico-8.gpl").to_string());
        self.load("psygnork".to_string(), include_str!("../../sys/assets/palettes/psygnork.gpl").to_string());
        self.load("smile-basic".to_string(), include_str!("../../sys/assets/palettes/smile-basic.gpl").to_string());
        self.load("solarized".to_string(), include_str!("../../sys/assets/palettes/solarized.gpl").to_string());
        self.load("teletext".to_string(), include_str!("../../sys/assets/palettes/teletext.gpl").to_string());
        self.load("vga-13h".to_string(), include_str!("../../sys/assets/palettes/vga-13h.gpl").to_string());
        self.load("web-safe-colors".to_string(), include_str!("../../sys/assets/palettes/web-safe-colors.gpl").to_string());
        self.load("win16".to_string(), include_str!("../../sys/assets/palettes/win16.gpl").to_string());
        self.load("x11".to_string(), include_str!("../../sys/assets/palettes/x11.gpl").to_string());
        self.load("zx-spectrum".to_string(), include_str!("../../sys/assets/palettes/zx-spectrum.gpl").to_string());
    }

    pub fn load(&mut self, name: String, data: String) {
        let buf_reader = Cursor::new(data);

        let mut values = Vec::new();

        for line in buf_reader.lines() {
            let line = line.unwrap();
            let l = line.trim_left().to_string();

            if l.len() == 0 {
                continue;
            }

            if l.starts_with("#") {
                continue;
            }

            let l_b = l.as_bytes();

            if ! (l_b[0] as char).is_digit(10) {
                continue;
            }

            let mut iter = l.split_whitespace();

            let r = iter.next().unwrap().parse::<u8>().unwrap();
            let g = iter.next().unwrap().parse::<u8>().unwrap();
            let b = iter.next().unwrap().parse::<u8>().unwrap();

            values.push(RGB::new(r, g, b));
        }

        self.palettes.insert(name.clone(), values);
        self.palettes_list.push(name.clone());
    }

    pub fn next(&mut self) {
        self.palette_idx = (self.palette_idx + 1) % self.palettes_list.len() as u32;
        let ref mut p_value = self.palettes_list[self.palette_idx as usize].clone();
        self.switch_to(p_value.clone());
    }

    pub fn switch_to(&mut self, name: String) {
        let ref values = *self.palettes.get(&name).unwrap();

        let mut idx = 0;
        for rgb_value in values {
            PALETTE.lock().unwrap().set_color(idx, rgb_value.r, rgb_value.g, rgb_value.b);
            idx += 1;
        }

        self.name = name.clone();
    }

    pub fn set_color(&mut self, color:u32, r: u8, g: u8, b: u8) {
        PALETTE.lock().unwrap().set_color(color, r, g, b);
    }

    pub fn get_color(&mut self, color:u32) -> u32 {
        PALETTE.lock().unwrap().get_color(color)
    }

    pub fn reset(&mut self) {
        PALETTE.lock().unwrap().reset();
    }
}

pub struct Px8New {
    pub screen: Arc<Mutex<gfx::Screen>>,
    pub palettes: Arc<Mutex<Palettes>>,
    pub cartridges: Vec<Cartridge>,
    pub current_cartridge: usize,
    pub lua_plugin: LuaPlugin,
    pub python_plugin: PythonPlugin,
    pub rust_plugin: Vec<Box<RustPlugin>>,
    pub code_type: Code,
    pub state: PX8State,
    pub menu: Menu,
    pub show_info_overlay: bool,
    pub fps: f64,
    pub draw_time: f64,
    pub init_time: f64,
    pub update_time: f64,
    pub record: Record,
    pub draw_return: bool,
    pub update_return: bool,
}


impl Px8New {
    pub fn new() -> Px8New {
        Px8New {
            screen: Arc::new(Mutex::new(gfx::Screen::new())),
            palettes: Arc::new(Mutex::new(Palettes::new())),
            cartridges: Vec::new(),
            current_cartridge: 0,
            lua_plugin: LuaPlugin::new(),
            python_plugin: PythonPlugin::new(),
            rust_plugin: Vec::new(),
            code_type: Code::UNKNOWN,
            state: PX8State::RUN,
            menu: Menu::new(),
            show_info_overlay: true,
            fps: 0.0,
            draw_time: 0.0,
            init_time: 0.0,
            update_time: 0.0,
            record: Record::new(),
            draw_return: true,
            update_return: true,
        }
    }

    pub fn init(&mut self) {
        self.palettes.lock().unwrap().init();
        self.palettes.lock().unwrap().switch_to("pico-8".to_string());

        self.screen.lock().unwrap().init();
        self.update_return = true;
        self.draw_return = true;
    }

    pub fn next_palette(&mut self) {
        self.palettes.lock().unwrap().next();
    }

    pub fn toggle_info_overlay(&mut self) {
        self.show_info_overlay = !self.show_info_overlay;
    }

    pub fn debug_update(&mut self) {
        if self.show_info_overlay {
            self.screen.lock().unwrap().rectfill(0, 0, SCREEN_WIDTH as i32, 8, 0);

            self.screen.lock().unwrap().force_print(format!("{:.0}FPS {:.2} {:.2} {:?}",
                                                            self.fps,
                                                            self.draw_time,
                                                            self.update_time,
                                                            &self.palettes.lock().unwrap().name).to_string(),
                                                    0, 0,
                                                    7);
        }
    }

    pub fn update(&mut self, players: Arc<Mutex<Players>>) -> bool {
        match self.state {
            PX8State::PAUSE => {
                if self.menu.stop() {
                    self.state = PX8State::RUN;
                }

                return self.menu.update(players);
            },
            PX8State::RUN => {
                if self.is_end() {
                    return false;
                }

                self.update_time = self.call_update(players) * 1000.0;
            }
        }

        return true;
    }

    pub fn draw(&mut self) {
        match self.state {
            PX8State::PAUSE => {
                self.menu.draw(self.screen.clone());
            },
            PX8State::RUN => {
                self.draw_time = self.call_draw() * 1000.0;

                if self.is_recording() {
                    self.record();
                }
            }
        }
    }

    pub fn is_end(&mut self) -> bool {
        return !self.update_return;
    }

    pub fn is_recording(&mut self) -> bool {
        return self.record.recording;
    }

    pub fn start_record(&mut self, filename: String) {
        info!("Start to record the frame");

        self.record.recording = true;
        self.record.images.clear();
        self.record.filename = filename;
    }

    pub fn record(&mut self) {
        info!("Recording the frame");

        if self.record.nb % 4 == 0 {
            let mut buffer: Vec<u8> = Vec::new();

            for x in 0..self::SCREEN_WIDTH {
                for y in 0..self::SCREEN_HEIGHT {
                    let value = self.screen.lock().unwrap().pget(x as u32, y as u32);
                    let rgb_value = PALETTE.lock().unwrap().get_rgb(value);

                    buffer.push(rgb_value.r);
                    buffer.push(rgb_value.g);
                    buffer.push(rgb_value.b);
                }
            }
            self.record.images.append(&mut buffer);
        }

        self.record.nb += 1;
    }

    pub fn stop_record(&mut self, scale: usize) {
        info!("Stop to record the frame {:?}", self.record.images.len());

        self.record.recording = false;

        let mut filedata = File::create(self.record.filename.clone()).unwrap();

        let mut encoder = gif::Encoder::new(&mut filedata, SCREEN_WIDTH as u16, SCREEN_HEIGHT as u16, &[]).unwrap();
        encoder.set(gif::Repeat::Infinite).unwrap();

        let mut idx = 0;
        for i in 0..self.record.images.len() / (SCREEN_WIDTH * SCREEN_HEIGHT * 3) {
            info!("Generate frame {:?} {:?}/{:?}", i, self.record.images.len(), idx);

            let mut buffer: Vec<u8> = Vec::new();

            for x in 0..SCREEN_WIDTH {
                for y in 0..SCREEN_HEIGHT {
                    buffer.push(*self.record.images.get(idx).unwrap());
                    buffer.push(*self.record.images.get(idx + 1).unwrap());
                    buffer.push(*self.record.images.get(idx + 2).unwrap());
                    idx += 3;
                }
            }

            info!("Creating ImageBuffer {:?}", buffer.len());

            let image = image::ImageBuffer::from_raw(SCREEN_WIDTH as u32,
                                                     SCREEN_HEIGHT as u32,
                                                     buffer).unwrap();

            info!("Rotating image");
            let image = image::DynamicImage::ImageRgb8(image).rotate90().resize(
                (SCREEN_WIDTH * scale) as u32,
                (SCREEN_HEIGHT * scale) as u32,
                image::FilterType::Nearest).fliph();

            info!("Creating gif Frame");
            let mut frame = gif::Frame::from_rgb((SCREEN_WIDTH * scale) as u16,
                                                 (SCREEN_HEIGHT * scale) as u16,
                                                 &mut *image.raw_pixels());

            frame.delay = 1;
            encoder.write_frame(&frame).unwrap();
        }

        info!("GIF created in {:?}", self.record.filename.clone());
    }

    pub fn screenshot(&mut self, filename: String) {
        info!("Taking screenshot in {:?}", filename);

        let mut buffer: Vec<u8> = Vec::new();

        for x in 0..SCREEN_WIDTH {
            for y in 0..SCREEN_HEIGHT {
                let value = self.screen.lock().unwrap().pget(x as u32, y as u32);
                let rgb_value = PALETTE.lock().unwrap().get_rgb(value);

                buffer.push(rgb_value.r);
                buffer.push(rgb_value.g);
                buffer.push(rgb_value.b);
            }
        }

        let image = image::ImageBuffer::from_raw(SCREEN_WIDTH as u32,
                                                 SCREEN_HEIGHT as u32,
                                                 buffer).unwrap();
        let image = image::DynamicImage::ImageRgb8(image).rotate270().resize(
            (SCREEN_WIDTH * 4) as u32,
            (SCREEN_WIDTH * 4) as u32,
            image::FilterType::Nearest).flipv();

        let mut output = File::create(&Path::new(&filename)).unwrap();
        image.save(&mut output, image::ImageFormat::PNG).unwrap();
    }

    pub fn save_current_cartridge(&mut self) {
        let ref mut cartridge = self.cartridges[self.current_cartridge];

        let output_filename = cartridge.filename.clone();
        info!("Saving the current cartridge in {:?}", output_filename);

        cartridge.gfx.set_sprites(self.screen.lock().unwrap().sprites.clone());

        match cartridge.format {
            CartridgeFormat::P8Format => {
                cartridge.save_in_p8(output_filename);
            },
            CartridgeFormat::PngFormat => {
                cartridge.save_in_p8(output_filename);
            },
            CartridgeFormat::Px8Format => {
                cartridge.save_data();
            }
        }
    }

    pub fn switch_pause(&mut self) {
        match self.state {
            PX8State::PAUSE => {
                self.state = PX8State::RUN;
                self.screen.lock().unwrap().restore();
            },
            PX8State::RUN => {
                self.menu.reset();
                self.state = PX8State::PAUSE;
                self.screen.lock().unwrap().save();
            }
        }
    }

    pub fn register<F: RustPlugin + 'static>(&mut self, callback: F) {
        self.rust_plugin.push(Box::new(callback));
    }

    pub fn load_cartridge(&mut self,
                          filename: String,
                          tx_input: Sender<Vec<u8>>,
                          rx_output: Receiver<Vec<u8>>,
                          players: Arc<Mutex<Players>>,
                          info: Arc<Mutex<Info>>,
                          sound: Arc<Mutex<Sound>>,
                          editor: bool,
                          mode: PX8Mode) -> bool {
        let idx = self.cartridges.len();

        info!("IDX CARTRIDGE {:?}", idx);

        if filename.contains(".png") {
            match Cartridge::from_png_file(filename.clone()) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the png cartridge {:?}", e),
            }
        } else if filename.contains(".p8") {
            match Cartridge::from_p8_file(filename.clone()) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the p8 cartridge {:?}", e),
            }
        } else if filename.contains(".py") {
            match Cartridge::from_p8_file(filename.clone()) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the p8 cartridge {:?}", e),
            }
        } else if filename.contains(".px8") {
            match Cartridge::from_px8_file(filename.clone()) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the px8 cartridge {:?}", e),
            }
        } else {
            panic!("Unknown file format !");
        }

        self.current_cartridge = idx;

        self.cartridges[idx].set_mode(mode == PX8Mode::PICO8);

        self.screen.lock().unwrap().set_sprites(self.cartridges[idx].gfx.sprites.clone());
        self.screen.lock().unwrap().set_sprites_flags(self.cartridges[idx].gff.flags.clone());

        self.screen.lock().unwrap().set_map(self.cartridges[idx].map.map);

        self.load_plugin(idx, tx_input, rx_output, players, info, sound, editor)
    }

    pub fn load_cartridge_raw(&mut self,
                              filename: String,
                              data: Vec<u8>,
                              tx_input: Sender<Vec<u8>>,
                              rx_output: Receiver<Vec<u8>>,
                              players: Arc<Mutex<Players>>,
                              info: Arc<Mutex<Info>>,
                              sound: Arc<Mutex<Sound>>,
                              editor: bool,
                              mode: PX8Mode) -> bool {
        let idx = self.cartridges.len();

        info!("IDX CARTRIDGE {:?}", idx);

        if filename.contains(".png") {
            match Cartridge::from_png_raw(filename.clone(), data) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the png cartridge"),
            }
        } else if filename.contains(".p8") {
            match Cartridge::from_p8_raw(filename.clone(), data) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the p8 cartridge"),
            }
        } else if filename.contains(".py") {
            match Cartridge::from_p8_raw(filename.clone(), data) {
                Ok(c) => self.cartridges.push(c),
                Err(e) => panic!("Impossible to load the p8 cartridge"),
            }
        } else {
            panic!("Unknown file");
        }

        self.current_cartridge = idx;

        self.cartridges[idx].set_mode(mode == PX8Mode::PICO8);

        self.screen.lock().unwrap().set_sprites(self.cartridges[idx].gfx.sprites.clone());
        self.screen.lock().unwrap().set_map(self.cartridges[idx].map.map);

        self.load_plugin(idx, tx_input, rx_output, players, info, sound, editor)
    }

    pub fn _get_code_type(&mut self, idx: usize) -> Code {
        if self.cartridges[idx].code.get_name() == "lua" {
            return Code::LUA;
        }

        if self.cartridges[idx].code.get_name() == "python" {
            return Code::PYTHON;
        }

        return Code::UNKNOWN;
    }

    pub fn switch_code(&mut self) {
        let idx = self.current_cartridge;

        let mut data;

        if self.cartridges[idx].edit {
            // Reload the code for the px8 format
            match self.cartridges[idx].format {
                CartridgeFormat::Px8Format => {
                    info!("Reloading code section for the cartridge");
                    self.cartridges[idx].code.reload();
                }
                _ => ()
            }

            data = self.cartridges[idx].code.get_data().clone();
            self.cartridges[idx].edit = false;
            self.code_type = self._get_code_type(idx);
        } else {
            data = self.load_editor("./sys/editor/editor.py".to_string()).clone();
            self.cartridges[idx].edit = true;
            self.code_type = Code::PYTHON;
        }

        debug!("CODE -> {:?}", data);

        match self.code_type {
            Code::LUA => {
                self.lua_plugin.load_code(data);
            },
            Code::PYTHON => {
                self.python_plugin.load_code(data);
            },
            _ => ()
        }
    }

    pub fn is_editing_current_cartridge(&mut self) -> bool {
        let idx = self.current_cartridge;
        return self.cartridges[idx].edit;
    }

    pub fn load_plugin(&mut self,
                       idx: usize,
                       tx_input: Sender<Vec<u8>>,
                       rx_output: Receiver<Vec<u8>>,
                       players: Arc<Mutex<Players>>,
                       info: Arc<Mutex<Info>>,
                       sound: Arc<Mutex<Sound>>,
                       editor: bool) -> bool {
        let mut data;

        info!("START TO LOAD THE PLUGIN");

        let gfx_sprites = self.cartridges[idx].gfx.sprites.clone();
        let gfx_map = self.cartridges[idx].map.map;

        self.code_type = self._get_code_type(idx);

        if editor {
            // Editor mode and original code type is different from Python
            match self.code_type {
                Code::LUA => {
                    info!("Loading LUA Plugin");
                    // load the lua plugin
                    self.lua_plugin.load(tx_input.clone(),
                                         rx_output.clone(),
                                         players.clone(),
                                         info.clone(),
                                         self.screen.clone());
                }
                _ => (),
            }

            data = self.load_editor("./sys/editor/editor.py".to_string()).clone();
            self.cartridges[idx].edit = true;
            self.code_type = Code::PYTHON;
        } else {
            data = self.cartridges[idx].code.get_data().clone();
        }

        debug!("CODE -> {:?}", data);

        match self.code_type {
            Code::LUA => {
                info!("Loading LUA Plugin");

                self.lua_plugin.load(tx_input.clone(),
                                     rx_output.clone(),
                                     players.clone(),
                                     info.clone(),
                                     self.screen.clone());

                return self.lua_plugin.load_code(data)
            },
            Code::PYTHON => {
                info!("Loading PYTHON Plugin");

                self.python_plugin.load(self.palettes.clone(),
                                        players.clone(),
                                        info.clone(),
                                        self.screen.clone(),
                                        sound.clone());

                return self.python_plugin.load_code(data)
            },
            _ => ()
        }

        false
    }

    pub fn load_editor(&mut self, filename: String) -> String {
        let mut data = "".to_string();

        let f = File::open(filename.clone()).unwrap();
        let mut buf_reader = BufReader::new(f);

        for line in buf_reader.lines() {
            let l = line.unwrap();

            data = data + "\n" + &l;
        }

        return data;
    }

    pub fn call_init(&mut self) -> f64 {
        let current_time = time::now();

        match self.code_type {
            Code::LUA       => self.lua_plugin.init(),
            Code::PYTHON    => self.python_plugin.init(),
            Code::RUST => {
                self.draw_return = true;
                for callback in self.rust_plugin.iter_mut() {
                    callback.init(self.screen.clone());
                }
            }
            _   => (),
        }

        let diff_time =  time::now() - current_time;
        let nanoseconds = (diff_time.num_nanoseconds().unwrap() as f64) - (diff_time.num_seconds() * 1000000000) as f64;
        let elapsed_time = diff_time.num_seconds() as f64 + nanoseconds/1000000000.0;

        return elapsed_time;
    }

    pub fn call_draw(&mut self) -> f64 {
        let current_time = time::now();

        match self.code_type {
            Code::LUA       => self.draw_return = self.lua_plugin.draw(),
            Code::PYTHON    => self.draw_return = self.python_plugin.draw(),
            Code::RUST => {
                self.draw_return = true;
                for callback in self.rust_plugin.iter_mut() {
                    callback.draw(self.screen.clone());
                }
            }
            _   => (),
        }

        let diff_time =  time::now() - current_time;
        let nanoseconds = (diff_time.num_nanoseconds().unwrap() as f64) - (diff_time.num_seconds() * 1000000000) as f64;
        let elapsed_time = diff_time.num_seconds() as f64 + nanoseconds/1000000000.0;

        return elapsed_time;
    }

    pub fn call_update(&mut self, players: Arc<Mutex<Players>>) -> f64 {
        let current_time = time::now();

        match self.code_type {
            Code::LUA       => self.update_return = self.lua_plugin.update(),
            Code::PYTHON    => self.update_return = self.python_plugin.update(),
            Code::RUST => {
                self.update_return = true;
                for callback in self.rust_plugin.iter_mut() {
                    callback.update(players.clone());
                }
            }
            _   => (),
        }

        let diff_time =  time::now() - current_time;
        let nanoseconds = (diff_time.num_nanoseconds().unwrap() as f64) - (diff_time.num_seconds() * 1000000000) as f64;
        let elapsed_time = diff_time.num_seconds() as f64 + nanoseconds/1000000000.0;

        return elapsed_time;
    }

}