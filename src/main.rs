extern crate sdl2;
extern crate sdl2_sys;
extern crate rand;


use std::vec::Vec;
use std::borrow::Borrow;

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


#[derive(Clone, Copy)]
enum TetrisCellColor {
    Red, Orange, Yellow, Green, Blue, DeepBlue, Purple,
}


impl TetrisCellColor {
    fn get_sdl_color(&self) -> Color {
        match self {
            &TetrisCellColor::Red => Color::RGB(200, 0, 0),
            &TetrisCellColor::Orange => Color::RGB(150, 100, 0),
            &TetrisCellColor::Yellow => Color::RGB(130, 130, 0),
            &TetrisCellColor::Green => Color::RGB(0, 200, 0),
            &TetrisCellColor::Blue => Color::RGB(0, 130, 130),
            &TetrisCellColor::DeepBlue => Color::RGB(0, 0, 200),
            &TetrisCellColor::Purple => Color::RGB(130, 0, 130),
        }
    }
}


#[derive(PartialEq, Clone, Copy)]
struct Point(usize, usize);

#[derive(PartialEq, Clone, Copy)]
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
}


trait CellScreenRenderer {
    fn render_cell_screen(&self, renderer: &mut Renderer);
}


impl <C: CellScreen> CellScreenRenderer for C {
    fn render_cell_screen(&self, renderer: &mut Renderer) {
        let PointOffset(x_max, y_max) = self.dimensions();
        let mut cells: Vec<Option<TetrisCellColor>> = Vec::with_capacity(x_max * y_max);

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
                    cells[y * layer_height + x] = layer_cell_iter.next().unwrap().clone();
                }
            }
        }

        let cells = cells;
        let mut cell_iter = cells.iter();
        let cell_size = self.cell_size();
        let cell_spacing = self.cell_spacing();

        renderer.set_draw_color(Color::RGB(0, 0, 0));
        renderer.clear();

        for y in 0 .. y_max {
            for x in 0 .. x_max {
                let cell = cell_iter.next().unwrap();
                renderer.set_draw_color(match cell {
                    &None => Color::RGB(0, 0, 0),
                    &Some(ref cell) => cell.get_sdl_color(),
                });
                renderer.fill_rect(sdl2::rect::Rect::new_unwrap(
                    (x * cell_size.0) as i32,
                    (y * cell_size.1) as i32,
                    (cell_size.0 - cell_spacing.0) as u32,
                    (cell_size.1 - cell_spacing.1) as u32,
                    ));
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
        self.cells[point.1 * dim.1 + point.0] = cell;
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
}


enum GameInputEvent {
    RotateClockwise,
    RotateCounterClockwise,
    MoveLeft,
    MoveRight,
    MoveDown,
    Timer,
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
    fn handle_event(&mut self, GameInputEvent, &mut Renderer) -> Option<()>;
}


struct RandomSquaresGame {
    cell_screen: SimpleCellScreen,
    counter: usize,
}


impl Game for RandomSquaresGame {
    fn handle_event(&mut self, _: GameInputEvent, renderer: &mut Renderer) -> Option<()> {
        self.counter = self.counter + 1;
        let screen_dim = self.cell_screen.dimensions();
        let mut cell_screen = &mut self.cell_screen;
        let mut number = self.counter;

        for y in 0 .. screen_dim.1 {
            for x in 0 .. screen_dim.0 {
                cell_screen.set_cell(Point(x, y), match number % 8 {
                    0 => None,
                    1 => Some(TetrisCellColor::Red),
                    2 => Some(TetrisCellColor::Orange),
                    3 => Some(TetrisCellColor::Yellow),
                    4 => Some(TetrisCellColor::Green),
                    5 => Some(TetrisCellColor::Blue),
                    6 => Some(TetrisCellColor::DeepBlue),
                    7 => Some(TetrisCellColor::Purple),
                    _ => panic!("lolwut"),
                });
                number = number + 1;
            }
        }

        cell_screen.render_cell_screen(renderer);
        renderer.present();
        Some(())
    }
}


fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let timer_subsystem = sdl_context.timer().unwrap();
    let event_subsystem = sdl_context.event().unwrap();
    let window = video_subsystem.window("Tetris", 800, 600).build().unwrap();
    let window_id = window.id();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut renderer = window.renderer().present_vsync().build().unwrap();

    let mut running = true;

    let mut game = RandomSquaresGame {
        cell_screen: SimpleCellScreen::new(),
        counter: 0,
    };

    let timer = timer_subsystem.add_timer(3000, Box::new(|| {
        event_subsystem.push_event(Event::User {
            code: 0,
            window_id: window_id,
            timestamp: 0,
            type_: SDL_USEREVENT,
        }).unwrap();
        3000
    }));

    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown {keycode: Some(Keycode::Q), ..} =>
                {
                    running = false;
                },
                e => match GameInputEvent::from_sdl_event(&e) {
                    Some(event) => {
                        match game.handle_event(event, &mut renderer) {
                            Some(..) => {},
                            None => {running = false},
                        }
                    },
                    None => {}
                }
            }
        }
    }

    drop(timer);
}
