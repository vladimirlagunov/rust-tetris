extern crate sdl2;
extern crate sdl2_sys;
extern crate rand;


use std::vec::Vec;
use std::borrow::Borrow;
use std::rc::Rc;
use std::marker::PhantomData;

use sdl2::pixels::Color;
use sdl2::surface::Surface;
use sdl2::video::{Window};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Renderer;
use sdl2_sys::event::SDL_USEREVENT;

use rand::random;


// struct Tetris<F: Figure + Sized> {
//     lines: Vec<Vec<Option<TetrisCellColor>>>,
//     current_figure: Option<(Point, F)>
// }


#[derive(Clone, Copy, Debug)]
enum TetrisCellColor {
    Red, Orange, Yellow, Green, Blue, DeepBlue, Purple,
}


impl TetrisCellColor {
    fn get_sdl_color(&self) -> Color {
        match self {
            &TetrisCellColor::Red => Color::RGB(200, 0, 0),
            &TetrisCellColor::Orange => Color::RGB(180, 130, 0),
            &TetrisCellColor::Yellow => Color::RGB(180, 180, 0),
            &TetrisCellColor::Green => Color::RGB(0, 200, 0),
            &TetrisCellColor::Blue => Color::RGB(0, 180, 180),
            &TetrisCellColor::DeepBlue => Color::RGB(0, 0, 200),
            &TetrisCellColor::Purple => Color::RGB(180, 0, 180),
        }
    }
}


#[derive(PartialEq, Clone, Copy, Debug)]
struct Point(usize, usize);

#[derive(PartialEq, Clone, Copy, Debug)]
struct PointOffset(usize, usize);


trait Figure {
    fn new_by_top_left_corner(Point, TetrisCellColor) -> Figure;
    fn rotate_clockwise(self) -> (PointOffset, Figure);
    fn rotate_counterclockwise(self) -> (PointOffset, Figure);
    fn lower_edge<'a>(&'a self) -> &'a [usize];
    fn dimensions(&self) -> PointOffset;
    fn color(&self) -> TetrisCellColor;
}


trait CellScreen {
    fn reset(&mut self);
    fn set_cell(&mut self, Point, Option<TetrisCellColor>);
    fn dimensions(&self) -> PointOffset;
    fn layers<'a>(&'a self) -> Vec<(Point, PointOffset, &'a [Option<TetrisCellColor>])>;
    fn cell_size(&self) -> PointOffset;
    fn cell_spacing(&self) -> PointOffset;
    fn window_size(&self) -> PointOffset;
    fn global_offset(&self) -> PointOffset;
}


trait CellScreenRenderer {
    fn render_cell_screen(&self, renderer: &mut Renderer);
}


impl <C: CellScreen> CellScreenRenderer for C {
    fn render_cell_screen(&self, renderer: &mut Renderer) {
        let PointOffset(x_glob_offset, y_glob_offset) = self.global_offset();

        let PointOffset(x_max, y_max) = self.dimensions();
        let mut cells: Vec<Option<TetrisCellColor>> = Vec::with_capacity(x_max * y_max);

        let cell_size = self.cell_size();
        let cell_spacing = self.cell_spacing();
        let window_size = self.window_size();

        renderer.set_draw_color(Color::RGB(0, 0, 0));
        renderer.clear();

        renderer.set_draw_color(Color::RGB(127, 127, 127));
        renderer.draw_rect(sdl2::rect::Rect::new_unwrap(
            x_glob_offset as i32,
            y_glob_offset as i32,
            (cell_size.0 * x_max) as u32,
            (cell_size.1 * y_max) as u32,
            ));

        for _ in 0 .. x_max * y_max {
            cells.push(None);
        }

        for layer_params in self.layers() {
            let (Point(layer_x0, layer_y0),
                 PointOffset(layer_width, layer_height),
                 layer_cells
                 ) = layer_params;

            assert!(layer_x0 + layer_width <= x_max);
            assert!(layer_y0 + layer_height <= y_max);

            let mut layer_cell_iter = layer_cells.iter();
            for y in layer_y0 .. layer_y0 + layer_height {
                for x in layer_x0 .. layer_x0 + layer_width {
                    cells[y * layer_width + x] = layer_cell_iter.next().unwrap().clone();
                    // println!("Updated (x={}, y={}), {} of {}, value={:?}",
                    //          x, y, y * layer_width + x, cells.len(),
                    //          cells[y * layer_width + x]);
                }
            }
        }

        let cells = cells;
        let mut cell_iter = cells.iter();

        for y in 0 .. y_max {
            for x in 0 .. x_max {
                let cell = cell_iter.next().unwrap();
                renderer.set_draw_color(match cell {
                    &None => Color::RGB(0, 0, 0),
                    &Some(ref cell) => cell.get_sdl_color(),
                });
                let rect = sdl2::rect::Rect::new_unwrap(
                    (x_glob_offset + x * cell_size.0 + cell_spacing.0) as i32,
                    (y_glob_offset + y * cell_size.1 + cell_spacing.1) as i32,
                    (cell_size.0 - cell_spacing.0 * 2) as u32,
                    (cell_size.1 - cell_spacing.1 * 2) as u32,
                    );
                renderer.fill_rect(rect);
            }
        }
    }
}


struct SimpleCellScreen {
    cells: [Option<TetrisCellColor>; 16 * 12],
    dim: PointOffset,
}


impl SimpleCellScreen {
    fn new() -> Self {
        SimpleCellScreen {
            cells: [None; 16 * 12],
            dim: PointOffset(16, 12),
        }
    }
}


impl CellScreen for SimpleCellScreen {
    fn reset(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = None;
        }
    }

    fn set_cell(&mut self, point: Point, cell: Option<TetrisCellColor>) {
        let dim = self.dimensions();
        self.cells[point.1 * dim.0 + point.0] = cell;
    }    

    fn dimensions(&self) -> PointOffset {
        self.dim.clone()
    }

    fn layers<'a>(&'a self) -> Vec<(Point, PointOffset, &'a [Option<TetrisCellColor>])> {
        vec![(
            Point(0, 0),
            self.dim.clone(),
            &self.cells,
            )]
    }

    fn cell_size(&self) -> PointOffset {
        PointOffset(50, 50)
    }

    fn cell_spacing(&self) -> PointOffset {
        PointOffset(2, 2)
    }

    fn global_offset(&self) -> PointOffset {
        PointOffset(10, 10)
    }

    fn window_size(&self) -> PointOffset {
        let off = self.global_offset();
        let cs = self.cell_size();
        let dim = self.dimensions();
        PointOffset(
            (off.0 * 2 + cs.0 * dim.0) as usize,
            (off.1 * 2 + cs.1 * dim.1) as usize,
            )
    }
}


enum GameInputEvent {
    RotateClockwise,
    RotateCounterClockwise,
    MoveLeft,
    MoveRight,
    MoveDown,
    Timer,
    Start,
}


impl GameInputEvent {
    fn from_sdl_event(event: &Event) -> Option<Self> {
        match event {
            &Event::User {code: 0, ..} => Some(GameInputEvent::Timer),
            &Event::KeyDown {keycode: Some(ref keycode), ..} => match keycode {
                &Keycode::Left => Some(GameInputEvent::MoveLeft),
                &Keycode::Right => Some(GameInputEvent::MoveRight),
                &Keycode::Space => Some(GameInputEvent::MoveDown),
                &Keycode::Up => Some(GameInputEvent::RotateClockwise),
                _ => None,
            },
            _ => None,
        }
    }
}


trait Game {
    fn new(&sdl2::Sdl) -> Self;
    fn run(&mut self, &mut sdl2::EventPump, &mut Renderer);
    fn window_size(&self) -> (u32, u32);
}


struct RandomSquaresGame {
    cell_screen: SimpleCellScreen,
    counter: usize,
    video: sdl2::VideoSubsystem,
    timer: sdl2::TimerSubsystem,
    event: sdl2::EventSubsystem,
}


fn timer_event() -> Event {
    Event::User {
        code: 0,
        window_id: 0,
        timestamp: 0,
        type_: SDL_USEREVENT,
    }    
}


fn is_timer_event(event: &Event) -> bool {
    if let &Event::User {code: 0, ..} = event { true } else { false }
}


impl Game for RandomSquaresGame {
    fn new(sdl: &sdl2::Sdl) -> Self {
        RandomSquaresGame {
            cell_screen: SimpleCellScreen::new(),
            counter: 0,
            video: sdl.video().unwrap(),
            timer: sdl.timer().unwrap(),
            event: sdl.event().unwrap(),
        }
    }

    fn window_size(&self) -> (u32, u32) {
        let ws = self.cell_screen.window_size();
        (ws.0 as u32, ws.1 as u32)
    }

    fn run(&mut self, event_pump: &mut sdl2::EventPump, renderer: &mut Renderer) {
        let event = self.event.clone();
        let timer = self.timer.clone();
        let timer = timer.add_timer(0, Box::new(move || {
            event.push_event(timer_event()).unwrap();
            500
        }));

        'game: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} => break 'game,
                    Event::KeyDown {keycode: Some(Keycode::Q), ..} => break 'game,
                    Event::KeyDown {keycode: Some(Keycode::Escape), ..} => break 'game,
                    event => match GameInputEvent::from_sdl_event(&event) {
                        Some(event) => if !self.handle_event(event, renderer) { break 'game },
                        None => {},
                    },
                }
            }
        }
    }
}

impl RandomSquaresGame {
    fn handle_event(&mut self, event: GameInputEvent, renderer: &mut Renderer) -> bool {
        self.counter = self.counter + 1;
        let screen_dim = self.cell_screen.dimensions();
        let mut cell_screen = &mut self.cell_screen;
        let mut number = self.counter;

        for y in 0 .. screen_dim.1 {
            for x in 0 .. screen_dim.0 {
                cell_screen.set_cell(Point(x, y), match number % 7 {
                    0 => Some(TetrisCellColor::Red),
                    1 => Some(TetrisCellColor::Orange),
                    2 => Some(TetrisCellColor::Yellow),
                    3 => Some(TetrisCellColor::Green),
                    4 => Some(TetrisCellColor::Blue),
                    5 => Some(TetrisCellColor::DeepBlue),
                    6 => Some(TetrisCellColor::Purple),
                    _ => panic!("lolwut"),
                });
                number = number + 1;
            }
        }

        cell_screen.render_cell_screen(renderer);
        renderer.present();

        true
    }
}


fn main() {
    let sdl_context = sdl2::init().unwrap();
    let event_subsystem = sdl_context.event().unwrap();

    let mut game = RandomSquaresGame::new(&sdl_context);
    let window_size = game.window_size();

    let window = sdl_context.video().unwrap().window("Tetris", window_size.0, window_size.1).build().unwrap();
    let window_id = window.id();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut renderer = window.renderer().build().unwrap();

    game.run(&mut event_pump, &mut renderer);
}
