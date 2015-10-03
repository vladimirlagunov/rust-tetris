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


const CELL_COUNT_X: usize = 10;
const CELL_COUNT_Y: usize = 16;
const PERIOD_MS: u32 = 200;


#[derive(Clone, Copy, Debug, PartialEq)]
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
struct Dimensions(usize, usize);

#[derive(PartialEq, Clone, Copy, Debug)]
struct PointOffset(isize, isize);



trait CellScreen {
    fn reset(&mut self);
    fn set_cell(&mut self, Point, Option<TetrisCellColor>);
    fn dimensions(&self) -> Dimensions;
    fn layers<'a>(&'a self) -> Vec<(Point, Dimensions, &'a [Option<TetrisCellColor>])>;
    fn cell_size(&self) -> Dimensions;
    fn cell_spacing(&self) -> Dimensions;
    fn window_size(&self) -> Dimensions;
    fn global_offset(&self) -> Dimensions;
}


trait CellScreenRenderer {
    fn render_cell_screen(&self, renderer: &mut Renderer);
}


impl <C: CellScreen> CellScreenRenderer for C {
    fn render_cell_screen(&self, renderer: &mut Renderer) {
        let Dimensions(x_glob_offset, y_glob_offset) = self.global_offset();

        let Dimensions(x_max, y_max) = self.dimensions();
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
                 Dimensions(layer_width, layer_height),
                 layer_cells
                 ) = layer_params;

            assert!(layer_x0 + layer_width <= x_max);
            assert!(layer_y0 + layer_height <= y_max);

            let mut layer_cell_iter = layer_cells.iter();
            for y in layer_y0 .. layer_y0 + layer_height {
                for x in layer_x0 .. layer_x0 + layer_width {
                    cells[y * x_max + x] = layer_cell_iter.next().unwrap().clone();
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
    cells: [Option<TetrisCellColor>; CELL_COUNT_X * CELL_COUNT_Y],
    dim: Dimensions,
}


impl SimpleCellScreen {
    fn new() -> Self {
        SimpleCellScreen {
            cells: [None; CELL_COUNT_X * CELL_COUNT_Y],
            dim: Dimensions(CELL_COUNT_X, CELL_COUNT_Y),
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

    fn dimensions(&self) -> Dimensions {
        self.dim.clone()
    }

    fn layers<'a>(&'a self) -> Vec<(Point, Dimensions, &'a [Option<TetrisCellColor>])> {
        vec![(
            Point(0, 0),
            self.dim.clone(),
            &self.cells,
            )]
    }

    fn cell_size(&self) -> Dimensions {
        Dimensions(40, 40)
    }

    fn cell_spacing(&self) -> Dimensions {
        Dimensions(2, 2)
    }

    fn global_offset(&self) -> Dimensions {
        Dimensions(10, 10)
    }

    fn window_size(&self) -> Dimensions {
        let off = self.global_offset();
        let cs = self.cell_size();
        let dim = self.dimensions();
        Dimensions(
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
    fn window_size(&self) -> (u32, u32) {
        let ws = self.cell_screen.window_size();
        (ws.0 as u32, ws.1 as u32)
    }

    fn run(&mut self, event_pump: &mut sdl2::EventPump, renderer: &mut Renderer) {
        let event = self.event.clone();
        let timer = self.timer.clone();
        let timer = timer.add_timer(PERIOD_MS, Box::new(move || {
            event.push_event(timer_event()).unwrap();
            PERIOD_MS
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
    fn new(sdl: &sdl2::Sdl) -> Self {
        RandomSquaresGame {
            cell_screen: SimpleCellScreen::new(),
            counter: 0,
            video: sdl.video().unwrap(),
            timer: sdl.timer().unwrap(),
            event: sdl.event().unwrap(),
        }
    }

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


struct TetrisCellScreen {
    cells: [Option<TetrisCellColor>; CELL_COUNT_X * CELL_COUNT_Y],
    dim: Dimensions,
    _figure: Option<(Point, TetrisCellColor, Figure)>,
    _figure_layer: Vec<Option<TetrisCellColor>>,
}


impl TetrisCellScreen {
    fn new() -> Self {
        TetrisCellScreen {
            cells: [None; CELL_COUNT_X * CELL_COUNT_Y],
            dim: Dimensions(CELL_COUNT_X, CELL_COUNT_Y),
            _figure: None,
            _figure_layer: Vec::new(),
        }
    }

    fn has_figure(&self) -> bool {
        ! self._figure.is_none()
    }

    fn get_figure(&self) -> Option<(Point, TetrisCellColor, Figure)> {
        self._figure.clone()
    }

    fn set_figure(&mut self, point: Point, color: TetrisCellColor, figure: Figure) {
        match &self._figure {
            &Some((_, old_color, ref old_figure))
                if old_color == color && old_figure == &figure => {},
            _ => {
                let bitmap = figure.bitmap();
                self._figure_layer.clear();
                for flag in bitmap.iter() {
                    self._figure_layer.push(
                        if *flag { Some(color) } else { None });
                }
            },
        };

        self._figure = Some((point, color, figure));
    }
}


impl CellScreen for TetrisCellScreen {
    fn reset(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = None;
        }
        self._figure = None;
    }

    fn set_cell(&mut self, point: Point, cell: Option<TetrisCellColor>) {
        let dim = self.dimensions();
        self.cells[point.1 * dim.0 + point.0] = cell;
    }

    fn dimensions(&self) -> Dimensions {
        self.dim.clone()
    }

    fn layers<'a>(&'a self) -> Vec<(Point, Dimensions, &'a [Option<TetrisCellColor>])> {
        let mut layers = Vec::with_capacity(2);
        layers.push((
            Point(0, 0),
            self.dim.clone(),
            self.cells.as_ref()));
        if let Some((ref point, _, ref figure)) = self.get_figure() {
            layers.push((
                point.clone(),
                figure.dimensions(),
                &self._figure_layer));
        }
        layers
    }

    fn cell_size(&self) -> Dimensions {
        Dimensions(40, 40)
    }

    fn cell_spacing(&self) -> Dimensions {
        Dimensions(2, 2)
    }

    fn global_offset(&self) -> Dimensions {
        Dimensions(10, 10)
    }

    fn window_size(&self) -> Dimensions {
        let off = self.global_offset();
        let cs = self.cell_size();
        let dim = self.dimensions();
        Dimensions(
            (off.0 * 2 + cs.0 * dim.0) as usize,
            (off.1 * 2 + cs.1 * dim.1) as usize,
            )
    }
}


struct TetrisGame<Random: rand::Rng> {
    cell_screen: TetrisCellScreen,
    dim: Dimensions,
    main_layer: [Option<TetrisCellColor>; CELL_COUNT_X * CELL_COUNT_Y],
    video: sdl2::VideoSubsystem,
    timer: sdl2::TimerSubsystem,
    event: sdl2::EventSubsystem,
    rng: Random,
}


impl <Random: rand::Rng> Game for TetrisGame<Random> {
    fn run(&mut self, event_pump: &mut sdl2::EventPump, renderer: &mut Renderer) {
        self.cell_screen.render_cell_screen(renderer);
        renderer.present();

        let event = self.event.clone();
        let timer = self.timer.clone();
        let timer = timer.add_timer(PERIOD_MS, Box::new(move || {
            event.push_event(timer_event()).unwrap();
            PERIOD_MS
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

    fn window_size(&self) -> (u32, u32) {
        let ws = self.cell_screen.window_size();
        (ws.0 as u32, ws.1 as u32)
    }
}


impl <Random: rand::Rng> TetrisGame<Random> {
    fn new(sdl: &sdl2::Sdl, rng: Random) -> Self {
        let mut game = TetrisGame {
            cell_screen: TetrisCellScreen::new(),
            dim: Dimensions(CELL_COUNT_X, CELL_COUNT_Y),
            main_layer: [None; CELL_COUNT_X * CELL_COUNT_Y],
            video: sdl.video().unwrap(),
            timer: sdl.timer().unwrap(),
            event: sdl.event().unwrap(),
            rng: rng,
        };
        let norm = game.create_new_figure();
        assert!(norm);

        game
    }

    fn create_new_figure(&mut self) -> bool {
        self.cell_screen._figure = None;
        let figure: Figure = self.rng.gen();
        let offset = figure.offset_from_top_center();
        assert!(offset.1 == 0);
        let dim = self.cell_screen.dimensions();
        let point = Point(
            (dim.0 as isize / 2 + offset.0) as usize,
            offset.1 as usize,
            );

        self.cell_screen.set_figure(point, figure.color(), figure);
        true
    }

    fn handle_event(&mut self, event: GameInputEvent, renderer: &mut Renderer) -> bool {
        match event {
            GameInputEvent::Timer => {
                let (point, color, figure) = self.cell_screen.get_figure().unwrap();
                if (point.1 + figure.dimensions().1 == self.cell_screen.dimensions().1) {
                    let fig_dim = figure.dimensions();
                    let mut new_cells = self.cell_screen._figure_layer.clone().into_iter();
                    for y in point.1 .. point.1 + fig_dim.1 {
                        for x in point.0 .. point.0 + fig_dim.0 {
                            self.cell_screen.set_cell(
                                Point(x, y),
                                new_cells.next().unwrap().clone());
                        }
                    }

                    if ! self.create_new_figure() {
                        return false;
                    }
                } else {
                    self.cell_screen.set_figure(Point(point.0, point.1 + 1), color, figure);
                }
            },
            GameInputEvent::MoveLeft => if self.cell_screen.has_figure() {
                self.move_figure_left();
            },
            GameInputEvent::MoveRight => if self.cell_screen.has_figure() {
                self.move_figure_right();
            },
            _ => {},
        }

        self.cell_screen.render_cell_screen(renderer);
        renderer.present();

        true
    }

    fn move_figure_left(&mut self) {
        let (mut point, color, figure) = self.cell_screen.get_figure().unwrap();
        if (point.0 > 0) {
            point.0 -= 1;
            self.cell_screen.set_figure(point, color, figure);
        }
    }

    fn move_figure_right(&mut self) {
        let (mut point, color, figure) = self.cell_screen.get_figure().unwrap();
        if (point.0 < self.cell_screen.dimensions().0 - figure.dimensions().0) {
            point.0 += 1;
            self.cell_screen.set_figure(point, color, figure);
        }
    }
}


#[derive(Clone, PartialEq, Debug)]
enum Figure {
    Cube,
    LineHorisontal,
    LineVertical,

    LeftFrame0,
    LeftFrame90,
    LeftFrame180,
    LeftFrame270,
    RightFrame0,
    RightFrame90,
    RightFrame180,
    RightFrame270,

    LeftZigzagHorizontal,
    LeftZigzagVertical,
    RightZigzagHorizontal,
    RightZigzagVertical,

    Pyramid0,
    Pyramid90,
    Pyramid180,
    Pyramid270,
}


const CUBE_CELLS: &'static [bool] = &[
    true, true,
    true, true,
    ];

const LINE_HORISONTAL: &'static [bool] = &[
    true, true, true, true
    ];

const LINE_VERTICAL: &'static [bool] = LINE_HORISONTAL;

const LEFT_FRAME_0: &'static [bool] = &[
    true,  true,
    false, true,
    false, true,
    ];

const LEFT_FRAME_90: &'static [bool] = &[
    false, false, true,
    true,  true,  true,
    ];

const LEFT_FRAME_180: &'static [bool] = &[
    true,  false,
    true,  false,
    true,  true,
    ];

const LEFT_FRAME_270: &'static [bool] = &[
    true,  true,  true,
    true,  false, false,
    ];

const RIGHT_FRAME_0: &'static [bool] = &[
    true, true, 
    true, false,
    true, false,
    ];

const RIGHT_FRAME_90: &'static [bool] = &[
    true,  true,  true,
    false, false, true,
    ];

const RIGHT_FRAME_180: &'static [bool] = &[
    false, true,
    false, true,
    true,  true,
    ];

const RIGHT_FRAME_270: &'static [bool] = &[
    true,  false, false,
    true,  true,  true,
    ];

const LEFT_ZIGZAG_HORIZONTAL: &'static [bool] = &[
    false, true, true,
    true,  true, false,
    ];

const LEFT_ZIGZAG_VERTICAL: &'static [bool] = &[
    true,  false,
    true,  true,
    false, true,
    ];

const RIGHT_ZIGZAG_HORIZONTAL: &'static [bool] = &[
    true,  true, false,
    false, true, true,
    ];

const RIGHT_ZIGZAG_VERTICAL: &'static [bool] = &[
    false, true,
    true,  true,
    true,  false,
    ];

const PYRAMID_0: &'static [bool] = &[
    false, true,  false,
    true,  true,  true,
    ];

const PYRAMID_90: &'static [bool] = &[
    true,  false,
    true,  true,
    true,  false,
    ];

const PYRAMID_180: &'static [bool] = &[
    true,  true,  true,
    false, true,  false,
    ];

const PYRAMID_270: &'static [bool] = &[
    false, true,
    true,  true,
    false, true,
    ];


impl Figure {
    fn offset_from_top_center(&self) -> PointOffset {
        match self {
            &Figure::Cube => PointOffset(-1, 0),
            &Figure::LineHorisontal => PointOffset(-2, 0),
            &Figure::LineVertical => PointOffset(0, 0),

            &Figure::LeftFrame0 => PointOffset(-1, 0),
            &Figure::LeftFrame90 => PointOffset(-2, 0),
            &Figure::LeftFrame180 => PointOffset(-1, 0),
            &Figure::LeftFrame270 => PointOffset(-2, 0),

            &Figure::RightFrame0 => PointOffset(-1, 0),
            &Figure::RightFrame90 => PointOffset(-2, 0),
            &Figure::RightFrame180 => PointOffset(-1, 0),
            &Figure::RightFrame270 => PointOffset(-2, 0),

            &Figure::LeftZigzagHorizontal => PointOffset(-1, 0),
            &Figure::LeftZigzagVertical => PointOffset(-1, 0),
            &Figure::RightZigzagHorizontal => PointOffset(-1, 0),
            &Figure::RightZigzagVertical => PointOffset(-1, 0),

            &Figure::Pyramid0 => PointOffset(-1, 0),
            &Figure::Pyramid90 => PointOffset(-1, 0),
            &Figure::Pyramid180 => PointOffset(-1, 0),
            &Figure::Pyramid270 => PointOffset(-1, 0),
        }
    }

    fn dimensions(&self) -> Dimensions {
        match self {
            &Figure::Cube => Dimensions(2, 2),
            &Figure::LineHorisontal => Dimensions(4, 1),
            &Figure::LineVertical => Dimensions(1, 4),

            &Figure::LeftFrame0 => Dimensions(2, 3),
            &Figure::LeftFrame90 => Dimensions(3, 2),
            &Figure::LeftFrame180 => Dimensions(2, 3),
            &Figure::LeftFrame270 => Dimensions(3, 2),
            &Figure::RightFrame0 => Dimensions(2, 3),
            &Figure::RightFrame90 => Dimensions(3, 2),
            &Figure::RightFrame180 => Dimensions(2, 3),
            &Figure::RightFrame270 => Dimensions(3, 2),

            &Figure::LeftZigzagHorizontal => Dimensions(2, 3),
            &Figure::LeftZigzagVertical => Dimensions(3, 2),
            &Figure::RightZigzagHorizontal => Dimensions(2, 3),
            &Figure::RightZigzagVertical => Dimensions(3, 2),

            &Figure::Pyramid0 => Dimensions(3, 2),
            &Figure::Pyramid90 => Dimensions(2, 3),
            &Figure::Pyramid180 => Dimensions(3, 2),
            &Figure::Pyramid270 => Dimensions(2, 3),
        }
    }

    fn color(&self) -> TetrisCellColor {
        match self {
            &Figure::Cube => TetrisCellColor::Red,
            &Figure::LineHorisontal => TetrisCellColor::Orange,
            &Figure::LineVertical => TetrisCellColor::Orange,

            &Figure::LeftFrame0 => TetrisCellColor::Yellow,
            &Figure::LeftFrame90 => TetrisCellColor::Yellow,
            &Figure::LeftFrame180 => TetrisCellColor::Yellow,
            &Figure::LeftFrame270 => TetrisCellColor::Yellow,
            &Figure::RightFrame0 => TetrisCellColor::Green,
            &Figure::RightFrame90 => TetrisCellColor::Green,
            &Figure::RightFrame180 => TetrisCellColor::Green,
            &Figure::RightFrame270 => TetrisCellColor::Green,

            &Figure::LeftZigzagHorizontal => TetrisCellColor::Blue,
            &Figure::LeftZigzagVertical => TetrisCellColor::Blue,
            &Figure::RightZigzagHorizontal => TetrisCellColor::DeepBlue,
            &Figure::RightZigzagVertical => TetrisCellColor::DeepBlue,

            &Figure::Pyramid0 => TetrisCellColor::Purple,
            &Figure::Pyramid90 => TetrisCellColor::Purple,
            &Figure::Pyramid180 => TetrisCellColor::Purple,
            &Figure::Pyramid270 => TetrisCellColor::Purple,
        }
    }

    fn bitmap(&self) -> &'static [bool] {
        match self {
            &Figure::Cube => CUBE_CELLS,
            &Figure::LineHorisontal => LINE_HORISONTAL,
            &Figure::LineVertical => LINE_VERTICAL,

            &Figure::LeftFrame0 => LEFT_FRAME_0,
            &Figure::LeftFrame90 => LEFT_FRAME_90,
            &Figure::LeftFrame180 => LEFT_FRAME_180,
            &Figure::LeftFrame270 => LEFT_FRAME_270,
            &Figure::RightFrame0 => RIGHT_FRAME_0,
            &Figure::RightFrame90 => RIGHT_FRAME_90,
            &Figure::RightFrame180 => RIGHT_FRAME_180,
            &Figure::RightFrame270 => RIGHT_FRAME_270,

            &Figure::LeftZigzagHorizontal => LEFT_ZIGZAG_HORIZONTAL,
            &Figure::LeftZigzagVertical => LEFT_ZIGZAG_VERTICAL,
            &Figure::RightZigzagHorizontal => RIGHT_ZIGZAG_HORIZONTAL,
            &Figure::RightZigzagVertical => RIGHT_ZIGZAG_VERTICAL,

            &Figure::Pyramid0 => PYRAMID_0,
            &Figure::Pyramid90 => PYRAMID_90,
            &Figure::Pyramid180 => PYRAMID_180,
            &Figure::Pyramid270 => PYRAMID_270,
        }
    }
}


impl rand::Rand for Figure {
    fn rand<R: rand::Rng>(rng: &mut R) -> Self {
        match rng.next_u32() % 14 {
            0 => Figure::Cube,
            1 => Figure::LineHorisontal,
            2 => Figure::LineVertical,

            3 => Figure::LeftFrame0,
            4 => Figure::LeftFrame90,
            5 => Figure::LeftFrame180,
            6 => Figure::LeftFrame270,
            7 => Figure::RightFrame0,
            8 => Figure::RightFrame90,
            9 => Figure::RightFrame180,
            10 => Figure::RightFrame270,

            11 => Figure::LeftZigzagHorizontal,
            12 => Figure::LeftZigzagVertical,
            13 => Figure::RightZigzagHorizontal,
            14 => Figure::RightZigzagVertical,

            15 => Figure::Pyramid0,
            16 => Figure::Pyramid90,
            17 => Figure::Pyramid180,
            18 => Figure::Pyramid270,

            _ => panic!("lolwut"),
        }
    }
}


fn main() {
    let sdl_context = sdl2::init().unwrap();
    let event_subsystem = sdl_context.event().unwrap();

    // let mut game = RandomSquaresGame::new(&sdl_context);
    let mut game = TetrisGame::new(&sdl_context, rand::thread_rng());
    let window_size = game.window_size();

    let window = sdl_context.video().unwrap().window("Tetris", window_size.0, window_size.1).build().unwrap();
    let window_id = window.id();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut renderer = window.renderer().build().unwrap();

    game.run(&mut event_pump, &mut renderer);
}
