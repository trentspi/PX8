use gfx::Screen;
use config::Players;
use std::sync::{Arc, Mutex};
use std::cmp::{max, min};
use std::collections::HashMap;

use px8::PX8Config;
use time;


pub fn point_in_rect(x: i32, y: i32, coord_x1: i32, coord_y1: i32, coord_x2: i32, coord_y2: i32) -> bool {
    (coord_x1 <= x && x < coord_x2) && (coord_y1 <= y && y < coord_y2)
}

pub struct State {
    mouse_x: i32,
    mouse_y: i32,
    mouse_state: u32,
    mouse_statep: u32,

    idx_sprites_batch: i32,
    idx_sprite_info: [i32; 2],
    idx_flag: [i32; 2],
    idx_sprite_number: [i32; 2],
    current_sprite: u32,

    x_zoom_sprite: u32,
    y_zoom_sprite: u32,
    zoom_sprite: u32,
    idx_zoom_sprite: u32,
    sprite_available_zooms: [u32; 3],

    idx_x_zoom_sprite: u32,
    idx_y_zoom_sprite: u32,

    idx_map: u32,

    on_current_sprite_x: u32,
    on_current_sprite_y: u32,
    on_current_sprite: bool,
}

impl State {
    pub fn new() -> State {
        State {
            mouse_x: 0,
            mouse_y: 0,
            mouse_state: 0,
            mouse_statep: 0,

            idx_sprites_batch: 196,
            idx_sprite_info: [129, 190],
            idx_flag: [140, 200],
            idx_sprite_number: [140, 208],
            current_sprite: 0,

            x_zoom_sprite: 0,
            y_zoom_sprite: 0,
            zoom_sprite: 1,
            idx_zoom_sprite: 0,
            sprite_available_zooms: [1, 2, 4],

            idx_x_zoom_sprite: 10,
            idx_y_zoom_sprite: 10,

            idx_map: 0,

            on_current_sprite_x: 0,
            on_current_sprite_y: 0,
            on_current_sprite: false,
        }
    }

    pub fn update(&mut self, players: Arc<Mutex<Players>>) {
        self.mouse_state = players.lock().unwrap().mouse_state();
        self.mouse_statep = players.lock().unwrap().mouse_state_quick();
        self.mouse_x = players.lock().unwrap().mouse_coordinate(0);
        self.mouse_y = players.lock().unwrap().mouse_coordinate(1);
    }
}

#[derive(Clone)]
pub struct Button {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: u32,
    text: String,
    highlight: bool,
    clicked: bool,
}

impl Button {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, color: u32, text: String, highlight: bool) -> Button {
        Button {
            x1: x1,
            y1: y1,
            x2: x2,
            y2: y2,
            color: color,
            text: text,
            highlight: highlight,
            clicked: highlight,
        }
    }

    pub fn update(&mut self, x: i32, y: i32) {
        self.clicked = (self.x1 <= x && x <= self.x2) && (self.y1 <= y && y <= self.y2);
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        screen.rectfill(self.x1, self.y1, self.x2, self.y2, self.color as i32);
        let mut color = 3;
        if self.clicked {
            color = 1;
        }
        screen.print(self.text.clone(), self.x1 + 1, self.y1, color);
    }

    pub fn is_click(&mut self) -> bool {
        self.clicked
    }
}

pub struct Widget {
    state: Arc<Mutex<State>>,
    name: String,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    w: u32,
    h: u32,
    data: Vec<u8>,
    highlight: HashMap<u32, u32>,
    clicked: bool,
}

impl Widget {
    pub fn new(state: Arc<Mutex<State>>, name: String, x: u32, y: u32, w: u32, h: u32, data: Vec<u8>, highlight: HashMap<u32, u32>) -> Widget {
        Widget {
            state: state,
            name: name,
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
            w: w,
            h: h,
            data: data,
            highlight: highlight,
            clicked: false
        }
    }

    pub fn is_click(&mut self) -> bool {
        self.clicked
    }

    pub fn update(&mut self) {
        let mouse_state = self.state.lock().unwrap().mouse_state;
        self.clicked = false;

        if mouse_state == 1 {
            let mouse_x = self.state.lock().unwrap().mouse_x as u32;
            let mouse_y = self.state.lock().unwrap().mouse_y as u32;

            self.clicked = (self.x1 <= mouse_x && mouse_x < self.x2) && (self.y1 <= mouse_y && mouse_y < self.y2);
        }
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        let mut idx_w = 0;
        let mut idx_h = 0;

        for pixel in &self.data {
            screen.pset((self.x1+idx_w) as i32, (self.y1+idx_h) as i32, *pixel as i32);

            idx_w += 1;
            if idx_w == self.w {
                idx_w = 0;
                idx_h += 1;
            }

        }
    }
}

pub struct PalettePicker {
    state: Arc<Mutex<State>>,
    idx_x: i32,
    idx_y: i32,
    current_color: u32,
    current_selection_x: i32,
    current_selection_y: i32,
}

impl PalettePicker {
    pub fn new(state: Arc<Mutex<State>>) -> PalettePicker {
        PalettePicker {
            state: state,
            idx_x: 165,
            idx_y: 16,
            current_color: 0,
            current_selection_x: 0,
            current_selection_y: 0,
        }
    }

    pub fn update(&mut self, screen: &mut Screen) {
        if self.state.lock().unwrap().mouse_statep == 1 {
            let mouse_x = self.state.lock().unwrap().mouse_x;
            let mouse_y = self.state.lock().unwrap().mouse_y;
            let idx_x_zoom_sprite = self.state.lock().unwrap().idx_x_zoom_sprite;
            let idx_y_zoom_sprite = self.state.lock().unwrap().idx_y_zoom_sprite;
            let zoom_sprite = self.state.lock().unwrap().zoom_sprite;

            if point_in_rect(mouse_x, mouse_y,
                             idx_x_zoom_sprite as i32,
                             idx_y_zoom_sprite as i32,
                             (idx_x_zoom_sprite + 128) as i32,
                             (idx_y_zoom_sprite + 128) as i32) {
                let idx_x = ((mouse_x - idx_x_zoom_sprite as i32) as f64 * zoom_sprite as f64 / 16.).floor() as u32;
                let idx_y = ((mouse_y - idx_y_zoom_sprite as i32) as f64 * zoom_sprite as f64 / 16.).floor() as u32;

                let x_zoom_sprite = self.state.lock().unwrap().x_zoom_sprite;
                let y_zoom_sprite = self.state.lock().unwrap().y_zoom_sprite;

                screen.sset(x_zoom_sprite + idx_x as u32, y_zoom_sprite + idx_y as u32, self.current_color as i32);
            }

            if point_in_rect(mouse_x, mouse_y,
                             self.idx_x,
                             self.idx_y,
                             self.idx_x + 4 * 16,
                             self.idx_y + 4 * 16) {
                let idx_x = (((mouse_x - self.idx_x) as f64).floor() / 16.) as i32;
                let idx_y = (((mouse_y - self.idx_y) as f64).floor() / 16.) as i32;

                self.current_color = (idx_x + idx_y * 4) as u32;
                self.current_selection_x = idx_x;
                self.current_selection_y = idx_y;
            }
        }
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        let mut idx = 0;
        let mut x = self.idx_x;
        let mut y = self.idx_y;

        for i in 0..16 {
            let pos_x = x + (16 * (idx % 4));

            let pos_y = y;
            screen.rectfill(pos_x, pos_y, pos_x + 15, pos_y + 15, i);
            idx += 1;

            if idx > 1 && idx % 4 == 0 {
                y += 16;
            }
        }

        let current_selection_x = (self.idx_x + 16*self.current_selection_x) - 1;
        let current_selection_y = (self.idx_y + 16*self.current_selection_y) - 1;

        screen.rect(current_selection_x, current_selection_y, current_selection_x+17, current_selection_y+17, 7);
    }
}

pub struct Flags {
    state: Arc<Mutex<State>>,
    values: [u32; 8],
    flags: HashMap<u32, u32>,
    idx_flag: [i32; 2],
    size: i32,
}

impl Flags {
    pub fn new(state: Arc<Mutex<State>>) -> Flags {
        let values = [0, 1, 2, 3, 4, 5, 6, 7];
        let mut flags = HashMap::new();

        let mut idx_flag = state.lock().unwrap().idx_flag;

        for i in values.iter() {
            flags.insert(*i, 0);
        }

        Flags {
            state: state.clone(),
            values: values,
            flags: flags,
            idx_flag: idx_flag,
            size: 4,
        }
    }

    pub fn update(&mut self, screen: &mut Screen) {
        let mut idx = 0;
        let idx_sprite = self.state.lock().unwrap().current_sprite;

        for i in self.values.iter() {
            let flag = screen.fget(idx_sprite, *i as u8);
            let mut color = 8;
            if flag {
                color = 11;
            }
            self.flags.insert(*i, color);

            let mouse_state = self.state.lock().unwrap().mouse_state;
            if mouse_state == 1 {
                let mouse_x = self.state.lock().unwrap().mouse_x;
                let mouse_y = self.state.lock().unwrap().mouse_y;

                if point_in_rect(mouse_x, mouse_y,
                                 self.idx_flag[0]+idx,
                                 self.idx_flag[1],
                                 self.idx_flag[0]+self.size+idx,
                                 self.idx_flag[1]+self.size) {
                    screen.fset(idx_sprite, *i as u8, !flag);
                }
            }

            idx += 8;
        }
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        let mut idx = 0;

        for k in self.values.iter() {
            let color = self.flags[k];

            screen.rectfill(self.idx_flag[0] + idx,
                            self.idx_flag[1],
                            self.idx_flag[0] + self.size + idx,
                            self.idx_flag[1] + self.size,
                            color as i32);

            idx += 8
        }
    }
}

#[derive(Debug)]
pub enum EditorState {
    SPRITE_EDITOR,
    MAP_EDITOR,
}

pub struct MapEditor {
    state: Arc<Mutex<State>>,
    coord: [i32; 4],
    offset_x: i32,
    offset_y: i32,
    available_zooms: [f32; 3],
    idx_zoom: u32,
    zoom: f32,
    cache: [u32; 128*32],
    select_field: [i32; 2],
    size_sprite: i32,
    current_sprite: [u32; 2],
    sprites_per_x: f32,
    sprites_per_y: f32,
}

impl MapEditor {
    pub fn new(state: Arc<Mutex<State>>) -> MapEditor {
        MapEditor {
            state: state.clone(),
            coord: [0, 8, 200, 182],
            offset_x: 0,
            offset_y: 0,
            available_zooms: [1., 0.5, 0.25],
            idx_zoom: 0,
            zoom: 1.,
            cache: [0; 128*32],
            select_field: [0, 8],
            size_sprite: 8,
            current_sprite: [0, 0],
            sprites_per_x: 25.,
            sprites_per_y: 22.,
        }
    }

    pub fn init(&mut self, screen: &mut Screen) {
        for y in 0..32 {
            for x in 0..128 {
                self.cache[x + y * 128] = screen.mget(x as i32, y as i32);
            }
        }
    }

    pub fn update(&mut self, players: Arc<Mutex<Players>>, screen: &mut Screen) {
        if players.lock().unwrap().btnp(0, 0) {
            self.offset_x -= 8;
            self.offset_x = max(0, self.offset_x);
        }

        if players.lock().unwrap().btnp(0, 1) {
            self.offset_x += 8;
            self.offset_x = min(((128./self.zoom - self.sprites_per_x * self.zoom) * self.zoom).floor() as i32, self.offset_x);
        }

        if players.lock().unwrap().btnp(0, 2) {
            self.offset_y -= 8;
            self.offset_y = max(0, self.offset_y);
        }

        if players.lock().unwrap().btnp(0, 3) {
            self.offset_y += 8;
            self.offset_y = min(((32./self.zoom - self.sprites_per_y * self.zoom) * self.zoom).floor() as i32, self.offset_y);
        }


        if players.lock().unwrap().btnp(0, 4) {
            self.idx_zoom = (self.idx_zoom + 1) % self.available_zooms.len() as u32;
            self.zoom = self.available_zooms[self.idx_zoom as usize];
            self.size_sprite = (8. * self.zoom).floor() as i32;
        }

        let mouse_x = self.state.lock().unwrap().mouse_x;
        let mouse_y = self.state.lock().unwrap().mouse_y;

        if point_in_rect(mouse_x, mouse_y,
                         self.coord[0], self.coord[1],
                         self.coord[2], self.coord[3]) {
            let mouse_statep = self.state.lock().unwrap().mouse_statep;

            let select_field_x = min(192, mouse_x - mouse_x % self.size_sprite);
            let select_field_y = min(176, mouse_y - mouse_y % self.size_sprite);

            let new_x = ((select_field_x + self.offset_x * self.size_sprite) as f64 / self.size_sprite as f64).floor() as u32;
            let new_y = ((select_field_y - self.coord[1] + self.offset_y * self.size_sprite) as f64 / self.size_sprite as f64).floor() as u32;
            if new_x < 128 && new_y < 32 {
                self.select_field[0] = select_field_x;
                self.select_field[1] = select_field_y;

                if mouse_statep == 1 {
                    let zoom_sprite = self.state.lock().unwrap().zoom_sprite;

                    for x in 0u32..zoom_sprite as u32 {
                        for y in 0u32..zoom_sprite as u32 {
                            let current_sprite = self.state.lock().unwrap().current_sprite + x + y * 16;

                            let idx = ((new_x + x) as f64 + (new_y + y) as f64 * 128.).floor() as usize;
                            self.cache[idx] = current_sprite;
                            screen.mset((new_x + x) as i32, (new_y + y) as i32, current_sprite);
                        }
                    }
                }
                self.current_sprite[0] = new_x;
                self.current_sprite[1] = new_y;
            }
        }
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        // clean screen
        screen.rectfill(self.coord[0], self.coord[1], self.coord[2], self.coord[3], 0);
        screen.rectfill(self.coord[2], self.coord[1], 240, self.coord[3], 5);

        // draw map
        let mut idx_y = 0;
        for y in self.offset_y as u32..min(32, self.offset_y as u32 + (self.sprites_per_y/self.zoom).floor() as u32) {
            let mut idx_x = 0;

            for x in self.offset_x as u32..min(128, self.offset_x as u32 + (self.sprites_per_x/self.zoom).floor() as u32) {
                let offset = x + y * 128;

                let sprite_number = self.cache[offset as usize];
                if sprite_number != 0 {
                    let sprite_x = (sprite_number % 16) * 8;
                    let sprite_y = (sprite_number as f32 / 16.).floor() as i32 * 8;

                    let dx = idx_x * ((8. * self.zoom).floor() as i32);
                    let dy = idx_y * ((8. * self.zoom).floor() as i32) + 9;
                    screen.sspr(sprite_x as u32, sprite_y as u32, 8, 8, dx, dy,
                                (self.zoom * 8.).floor() as u32,
                                (self.zoom * 8.).floor() as u32,
                                false, false);
                }

                idx_x += 1
            }

            idx_y += 1
        }


        // draw selected sprites
        let zoom_sprite = self.state.lock().unwrap().zoom_sprite;

        screen.rect(self.select_field[0],
                    self.select_field[1],
                    self.select_field[0] + (8. * self.zoom * zoom_sprite as f32).floor() as i32,
                    self.select_field[1] + (8. * self.zoom * zoom_sprite as f32).floor() as i32,
                    7);

        // Draw info
        screen.print(format!("{:?} {:?}: {:?}",
                             self.current_sprite[0],
                             self.current_sprite[1],
                             self.cache[(self.current_sprite[0] + self.current_sprite[1] * 128) as usize]),
                     0, 0, 7);
    }
}

pub struct SpriteEditor {
    state: Arc<Mutex<State>>,
    pp: PalettePicker,
}

impl SpriteEditor {
    pub fn new(state: Arc<Mutex<State>>) -> SpriteEditor {
        SpriteEditor {
            state: state.clone(),
            pp: PalettePicker::new(state.clone()),
        }
    }

    pub fn update(&mut self, players: Arc<Mutex<Players>>, screen: &mut Screen) {
        self.pp.update(screen);

        if players.lock().unwrap().btnp(0, 4) {
            let idx_zoom_sprite = self.state.lock().unwrap().idx_zoom_sprite;
            let sprite_available_zooms = self.state.lock().unwrap().sprite_available_zooms;

            let new_idx_zoom_sprite = (idx_zoom_sprite + 1) % sprite_available_zooms.len() as u32;
            self.state.lock().unwrap().idx_zoom_sprite = new_idx_zoom_sprite;

            self.state.lock().unwrap().zoom_sprite = sprite_available_zooms[new_idx_zoom_sprite as usize];
        }
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        self.pp.draw(screen);

        let x_zoom_sprite = self.state.lock().unwrap().x_zoom_sprite;
        let y_zoom_sprite = self.state.lock().unwrap().y_zoom_sprite;
        let zoom_sprite = self.state.lock().unwrap().zoom_sprite;
        let idx_x_zoom_sprite = self.state.lock().unwrap().idx_x_zoom_sprite;
        let idx_y_zoom_sprite = self.state.lock().unwrap().idx_y_zoom_sprite;


        screen.sspr(x_zoom_sprite,
                    y_zoom_sprite,
                    8*zoom_sprite as u32,
                    8*zoom_sprite as u32,
                    idx_x_zoom_sprite as i32,
                    idx_y_zoom_sprite as i32,
                    128,
                    128,
                    false,
                    false);
    }
}

pub struct SpritesMap {
    state: Arc<Mutex<State>>,
    buttons: [i32; 4],
    buttons_map: Vec<Arc<Mutex<Button>>>,
    flags: Flags,
}

impl SpritesMap {
    pub fn new(state: Arc<Mutex<State>>) -> SpritesMap {
        let mut buttons_map = Vec::new();
        buttons_map.push(Arc::new(Mutex::new(Button::new(208, 191, 212, 199, 2, "1".to_string(), true))));
        buttons_map.push(Arc::new(Mutex::new(Button::new(213, 191, 217, 199, 2, "2".to_string(), false))));
        buttons_map.push(Arc::new(Mutex::new(Button::new(218, 191, 222, 199, 2, "3".to_string(), false))));
        buttons_map.push(Arc::new(Mutex::new(Button::new(223, 191, 228, 199, 2, "4".to_string(), false))));

        SpritesMap {
            state: state.clone(),
            buttons: [208, 191, 228, 199],
            buttons_map: buttons_map.clone(),
            flags: Flags::new(state.clone()),
        }
    }

    pub fn update(&mut self, screen: &mut Screen) {
        self.state.lock().unwrap().on_current_sprite = false;

        if self.state.lock().unwrap().mouse_state == 1 {
            let mouse_x = self.state.lock().unwrap().mouse_x;
            let mouse_y = self.state.lock().unwrap().mouse_y;

            if point_in_rect(mouse_x, mouse_y,
                             self.buttons[0], self.buttons[1],
                             self.buttons[2], self.buttons[3]) {
                let mut btn_idx = 0;
                for button in &self.buttons_map {
                    let mut button = button.lock().unwrap();
                    button.update(mouse_x, mouse_y);
                    if button.is_click() {
                        self.state.lock().unwrap().idx_map = btn_idx;
                        self.state.lock().unwrap().current_sprite = 64 * btn_idx;
                    }

                    btn_idx += 1;
                }
            }


            let idx_sprites_batch = self.state.lock().unwrap().idx_sprites_batch;

            if (mouse_y >= self.state.lock().unwrap().idx_sprites_batch) && mouse_y < 232 && mouse_x <= 128 {
                let y = ((mouse_y - idx_sprites_batch) as f64 / 8.).floor() as u32;
                let x = (mouse_x as f64 / 8.).floor() as u32;

                let idx_map = self.state.lock().unwrap().idx_map;
                self.state.lock().unwrap().current_sprite = (x + y * 16) + 64 * idx_map;

                let current_sprite = self.state.lock().unwrap().current_sprite;
                self.state.lock().unwrap().x_zoom_sprite = (current_sprite % 16) * 8;
                self.state.lock().unwrap().y_zoom_sprite = (current_sprite as f64 / 16.).floor() as u32 * 8;
            }
        }

        // Update flags
        self.flags.update(screen);
    }

    pub fn draw(&mut self, screen: &mut Screen) {
        self.draw_sprite_map(screen);
        self.draw_sprite_flags(screen);
        self.draw_button(screen);
        self.draw_information(screen);
    }

    pub fn draw_sprite_flags(&mut self, screen: &mut Screen) {
        self.flags.draw(screen);
    }

    pub fn draw_sprite_map(&mut self, screen: &mut Screen) {
        let zoom = self.state.lock().unwrap().zoom_sprite as i32;
        let mut current_sprite_x = 0;
        let mut current_sprite_y = 0;

        let mut idx = self.state.lock().unwrap().idx_map * (4 * 16);
        let mut y = self.state.lock().unwrap().idx_sprites_batch;
        for j in 0..4 {
            let mut x = 0;
            for _ in 0..16 {
                screen.spr(idx, x, y, 1, 1, false, false);
                if idx == self.state.lock().unwrap().current_sprite {
                    current_sprite_x = x;
                    current_sprite_y = y;
                }
                x += 8;
                idx += 1;
            }
            y += 8;
        }

        current_sprite_x -= 1;
        screen.rect(current_sprite_x, current_sprite_y, current_sprite_x+8*zoom, current_sprite_y+8*zoom, 7);
        screen.rect(current_sprite_x - 1, current_sprite_y - 1, current_sprite_x+1+8*zoom, current_sprite_y+1+8*zoom, 0);
    }

    pub fn draw_button(&mut self, screen: &mut Screen) {
        for button in &self.buttons_map {
            button.lock().unwrap().draw(screen);
        }
    }

    pub fn draw_information(&mut self, screen: &mut Screen) {
        if self.state.lock().unwrap().on_current_sprite {
            let on_current_sprite_x = self.state.lock().unwrap().on_current_sprite_x;
            let on_current_sprite_y = self.state.lock().unwrap().on_current_sprite_y;

            screen.print(format!("{:?},{:?}", on_current_sprite_x, on_current_sprite_y),
                         0, 232,
                         5);
        }

        let idx_sprite_number = self.state.lock().unwrap().idx_sprite_number;
        screen.print(format!("{:?}", self.state.lock().unwrap().current_sprite),
                     idx_sprite_number[0], idx_sprite_number[1],
                     7);
    }
}

pub struct Editor {
    state: Arc<Mutex<State>>,
    state_editor: EditorState,
    sm: SpritesMap,
    me: MapEditor,
    se: SpriteEditor,
    widgets: Vec<Arc<Mutex<Widget>>>,
}

impl Editor {
    pub fn new() -> Editor {
        let state = Arc::new(Mutex::new(State::new()));

        let mut widgets = Vec::new();

        widgets.push(Arc::new(Mutex::new(Widget::new(state.clone(), "SPRITES".to_string(), 222, 1, 8, 6,
                                                       vec![6, 11, 11, 11, 11, 11, 11, 6,
                                                            11, 6, 6, 6, 6, 6, 6, 11,
                                                            11, 6, 11, 11, 11, 11, 6, 11,
                                                            11, 6, 11, 11, 11, 11, 6, 11,
                                                            11, 6, 6, 6, 6, 6, 6, 11,
                                                            6, 11, 11, 11, 11, 11, 11, 6],
                                                        HashMap::new()))));
        widgets.push(Arc::new(Mutex::new(Widget::new(state.clone(), "MAP".to_string(), 231, 1, 8, 6,
                                                     vec![11, 11, 11, 11, 11, 11, 11, 11,
                                                          11, 6, 6, 6, 6, 6, 6, 11,
                                                          11, 6, 11, 11, 11, 11, 6, 11,
                                                          11, 6, 11, 11, 11, 11, 6, 11,
                                                          11, 6, 6, 6, 6, 6, 6, 11,
                                                          11, 11, 11, 11, 11, 11, 11, 11],
                                                     HashMap::new()))));

        Editor {
            state: state.clone(),
            state_editor: EditorState::SPRITE_EDITOR,
            sm: SpritesMap::new(state.clone()),
            me: MapEditor::new(state.clone()),
            se: SpriteEditor::new(state.clone()),
            widgets: widgets,
        }
    }

    pub fn init(&mut self, config: Arc<Mutex<PX8Config>>, screen: &mut Screen) {
        info!("[EDITOR] Init");
        config.lock().unwrap().toggle_mouse(true);

        screen.mode(240, 236, 1.);
    }

    pub fn update(&mut self, players: Arc<Mutex<Players>>) -> bool {

        true
    }

    pub fn draw(&mut self, players: Arc<Mutex<Players>>, screen: &mut Screen) -> f64 {
        let current_time = time::now();

        screen.cls();

        self.state.lock().unwrap().update(players.clone());
        self.sm.update(screen);
        match self.state_editor {
            EditorState::SPRITE_EDITOR => { self.se.update(players.clone(), screen); }
            EditorState::MAP_EDITOR => { self.me.update(players.clone(), screen); }
        }

        for widget in &self.widgets {
            widget.lock().unwrap().update();
        }

        for widget in &self.widgets {
            let is_click = widget.lock().unwrap().is_click();
            if is_click {
                if widget.lock().unwrap().name == "SPRITES" {
                    self.state_editor = EditorState::SPRITE_EDITOR;
                }
                if widget.lock().unwrap().name == "MAP" {
                    self.state_editor = EditorState::MAP_EDITOR;
                    self.me.init(screen);
                }
            }
        }

        let width = screen.mode_width() as i32;
        let height = screen.mode_height() as i32;

        let idx_sprites_batch = self.state.lock().unwrap().idx_sprites_batch;
        let idx_sprite_info = self.state.lock().unwrap().idx_sprite_info;

        // Draw contour
        screen.rectfill(0, 0, width, 8, 11);
        screen.rectfill(0, height-8, width, height, 11);
        screen.rectfill(0, 139, width, idx_sprites_batch-1, 5);
        screen.rectfill(0, 9, 8, 189, 5);
        screen.rectfill(139, 9, width, 190, 5);
        screen.rectfill(idx_sprite_info[0], idx_sprite_info[1], width, height-9, 5);

        // Draw sprites map
        self.sm.draw(screen);

        // Draw sprite or map editor
        match self.state_editor {
            EditorState::SPRITE_EDITOR => { self.se.draw(screen); }
            EditorState::MAP_EDITOR => { self.me.draw(screen); }
        }



        for widget in &self.widgets {
            widget.lock().unwrap().draw(screen);
        }

        let diff_time = time::now() - current_time;
        let nanoseconds = (diff_time.num_nanoseconds().unwrap() as f64) -
            (diff_time.num_seconds() * 1000000000) as f64;

        // Elapsed time
        diff_time.num_seconds() as f64 + nanoseconds / 1000000000.0
    }
}
