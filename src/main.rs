extern crate sdl2;
extern crate sdl2_sys;
extern crate rand;
extern crate time;


use std::cmp::{min, max};
use std::vec::Vec;

use sdl2::pixels::Color;
use sdl2::keyboard::Scancode;
use sdl2::render::Renderer;


const CELL_COUNT_X: usize = 10;
const CELL_COUNT_Y: usize = 16;


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
        let mut cells = std::iter::repeat(None).take(x_max * y_max).collect::<Vec<_>>();

        let cell_size = self.cell_size();
        let cell_spacing = self.cell_spacing();

        renderer.set_draw_color(Color::RGB(0, 0, 0));
        renderer.clear();

        renderer.set_draw_color(Color::RGB(127, 127, 127));
        renderer.draw_rect(sdl2::rect::Rect::new_unwrap(
            x_glob_offset as i32,
            y_glob_offset as i32,
            (cell_size.0 * x_max) as u32,
            (cell_size.1 * y_max) as u32,
            ));

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
                    if let Some(color) = layer_cell_iter.next().unwrap().clone() {
                        cells[y * x_max + x] = Some(color);
                    }
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


enum GameInputEvent {
    RotateClockwise,
    MoveLeft,
    MoveRight,
    MoveDown,
    Timer,
}


trait Game {
    fn run(&mut self, &mut sdl2::EventPump, &mut Renderer);
    fn window_size(&self) -> (u32, u32);
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
    rng: Random,
    figures_generated: usize,
}


fn precise_time_ms() -> u64 {
    time::precise_time_ns() / 1_000_000
}


impl <Random: rand::Rng> Game for TetrisGame<Random> {
    fn run(&mut self, event_pump: &mut sdl2::EventPump, renderer: &mut Renderer) {
        let mut is_paused = false;
        let mut running = true;
        let mut pause_was_pressed = false;

        const LOOP_PERIOD_MS: u32 = 10;
        const MOVE_PERIOD_MS: u64 = 120;
        let mut last_move_time_ms = None;

        let mut auto_move_down_period = 500;
        let mut last_auto_move_down_ms = None;

        let mut rotate_was_pressed = false;
        let mut move_down_was_pressed = false;

        const SPEED_UP_AFTER_FIGURE_COUNT: usize = 100;
        let mut last_speed_up_was_at_figure = 0;

        loop {
            self.cell_screen.render_cell_screen(renderer);
            renderer.present();

            event_pump.wait_event_timeout(LOOP_PERIOD_MS);

            let keycodes = event_pump.keyboard_state();
            let current_time_ms = precise_time_ms();

            if keycodes.is_scancode_pressed(Scancode::Q)
                || keycodes.is_scancode_pressed(Scancode::Escape)
            { break }

            if ! running { continue }

            if keycodes.is_scancode_pressed(Scancode::P) {
                if ! pause_was_pressed {
                    is_paused = ! is_paused;
                    pause_was_pressed = true;
                }
            } else {
                pause_was_pressed = false;
            }

            if is_paused { continue }

            let move_left_pressed = keycodes.is_scancode_pressed(Scancode::Left);
            let move_right_pressed = keycodes.is_scancode_pressed(Scancode::Right);
            let rotate_pressed = keycodes.is_scancode_pressed(Scancode::Up);
            let move_down_pressed =
                keycodes.is_scancode_pressed(Scancode::Down)
                || keycodes.is_scancode_pressed(Scancode::Space);

            drop(keycodes);

            if move_left_pressed || move_right_pressed {
                if last_move_time_ms.is_none()
                    || last_move_time_ms.unwrap() + MOVE_PERIOD_MS <= current_time_ms
                {
                    let event = if move_left_pressed {
                        GameInputEvent::MoveLeft
                    } else {
                        GameInputEvent::MoveRight
                    };
                    running = self.handle_event(event);
                    last_move_time_ms = Some(precise_time_ms())
                }
            } else {
                last_move_time_ms = None;
            }

            if rotate_pressed {
                if ! rotate_was_pressed {
                    self.handle_event(GameInputEvent::RotateClockwise);
                    rotate_was_pressed = true;
                }   
            } else {
                rotate_was_pressed = false;
            }

            if move_down_pressed {
                if ! move_down_was_pressed {
                    running = self.handle_event(GameInputEvent::MoveDown);
                    move_down_was_pressed = true;
                }
            } else {
                move_down_was_pressed = false;
            }

            if last_speed_up_was_at_figure + SPEED_UP_AFTER_FIGURE_COUNT <= self.figures_generated {
                last_speed_up_was_at_figure = self.figures_generated;
                auto_move_down_period = auto_move_down_period * 3 / 4;
            }

            if last_auto_move_down_ms.is_none() {
                last_auto_move_down_ms = Some(current_time_ms);
            } else if last_auto_move_down_ms.unwrap() + auto_move_down_period <= current_time_ms {
                running = self.handle_event(GameInputEvent::Timer);
                last_auto_move_down_ms = Some(current_time_ms);
            }
        }
    }

    fn window_size(&self) -> (u32, u32) {
        let ws = self.cell_screen.window_size();
        (ws.0 as u32, ws.1 as u32)
    }
}


impl <Random: rand::Rng> TetrisGame<Random> {
    fn new(rng: Random) -> Self {
        let mut game = TetrisGame {
            cell_screen: TetrisCellScreen::new(),
            rng: rng,
            figures_generated: 0,
        };
        let can_create_first_figure = game.create_new_figure();
        assert!(can_create_first_figure);
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

        self.figures_generated += 1;

        if self._figure_overlaps_cells(&point, &figure) {
            false
        } else {
            self.cell_screen.set_figure(point, figure.color(), figure);
            true
        }
    }

    fn handle_event(&mut self, event: GameInputEvent) -> bool {
        let recreate_figure: bool = match event {
            GameInputEvent::Timer => {
                ! self._try_move_figure_down()
            },
            GameInputEvent::MoveLeft => {
                if self.cell_screen.has_figure() { self.move_figure_left() }
                false
            },
            GameInputEvent::MoveRight => {
                if self.cell_screen.has_figure() { self.move_figure_right() }
                false
            },
            GameInputEvent::MoveDown => {
                if self.cell_screen.has_figure() { self.move_figure_down() }
                true
            },
            GameInputEvent::RotateClockwise => {
                if self.cell_screen.has_figure() {
                    self.rotate_clockwise();
                }
                false
            },
        };

        if recreate_figure {
            self.remove_filled_lines();

            if ! self.create_new_figure() {
                return false;
            }
        }

        true
    }

    fn move_figure_left(&mut self) {
        let (mut point, color, figure) = self.cell_screen.get_figure().unwrap();
        if point.0 > 0 {
            point.0 -= 1;
            if ! self._figure_overlaps_cells(&point, &figure) {
                self.cell_screen.set_figure(point, color, figure);
            }
        }
    }

    fn move_figure_right(&mut self) {
        let (mut point, color, figure) = self.cell_screen.get_figure().unwrap();
        if point.0 < self.cell_screen.dimensions().0 - figure.dimensions().0 {
            point.0 += 1;
            if ! self._figure_overlaps_cells(&point, &figure) {
                self.cell_screen.set_figure(point, color, figure);
            }
        }
    }

    fn move_figure_down(&mut self) {
        while self._try_move_figure_down() {}
    }

    fn _try_move_figure_down(&mut self) -> bool {
        let (point, color, figure) = self.cell_screen.get_figure().unwrap();
            let fig_dim = figure.dimensions();

        let can_go_down =
            (point.1 + figure.dimensions().1) < self.cell_screen.dimensions().1
            && ! self._figure_overlaps_cells(&Point(point.0, point.1 + 1), &figure);

        if can_go_down {
            self.cell_screen.set_figure(Point(point.0, point.1 + 1), color, figure);
            true
        } else {
            let mut new_cells = self.cell_screen._figure_layer.clone().into_iter();
            for y in point.1 .. point.1 + fig_dim.1 {
                for x in point.0 .. point.0 + fig_dim.0 {
                    if let Some(color) = new_cells.next().unwrap().clone() {
                        self.cell_screen.set_cell(Point(x, y), Some(color));
                    }
                }
            }
            false
        }
    }

    fn _figure_overlaps_cells(&self, new_point: &Point, figure: &Figure) -> bool {
        let figure_bitmap = figure.bitmap();
        let existing_cells = self.cell_screen.cells;
        let fig_dim = figure.dimensions();
        let screen_dim = self.cell_screen.dimensions();

        for y in 0 .. fig_dim.1 {
            for x in 0 .. fig_dim.0 {
                let screen_offset_in_new_point = (new_point.1 + y) * screen_dim.0 + new_point.0 + x;
                let is_cell_in_figure = figure_bitmap[y * fig_dim.0 + x];
                let has_cell_here = ! existing_cells[screen_offset_in_new_point].is_none();
                if is_cell_in_figure && has_cell_here {
                    return true;
                }
            }
        }
        false
    }

    fn rotate_clockwise(&mut self) {
        let (point, color, figure) = self.cell_screen.get_figure().unwrap();
        let (offset, rotated_figure) = figure.rotate_clockwise();

        let new_x = (max(0, point.0 as isize + offset.0)) as usize;
        let new_y = (max(0, point.1 as isize + offset.1)) as usize;
        let dim = self.cell_screen.dimensions();
        let fig_dim = rotated_figure.dimensions();

        let new_x = min(new_x, dim.0 - fig_dim.0);
        let new_y = min(new_y, dim.1 - fig_dim.1);

        self.cell_screen.set_figure(
            Point(new_x, new_y),
            color,
            rotated_figure,
            );
    }

    fn remove_filled_lines(&mut self) {
        let dim = self.cell_screen.dimensions();

        let mut any_filled_line = false;
        let mut filled_lines = Vec::with_capacity(dim.1);
        let mut cell_position = 0;
        for _ in 0 .. dim.1 {
            let mut filled_line = true;
            for _ in 0 .. dim.0 {
                filled_line &= ! self.cell_screen.cells[cell_position].is_none();
                cell_position += 1;
            }
            filled_lines.push(filled_line);
            any_filled_line |= filled_line;
        }

        assert!(cell_position == dim.0 * dim.1);

        if ! any_filled_line {
            return;
        }

        let mut offset = 0;
        for line in 0 .. dim.1 {
            let line = dim.1 - line;

            if line == filled_lines.len() {
                while filled_lines.pop().unwrap() {
                    offset += dim.0;
                }
            }

            for _ in 0 .. dim.0 {
                cell_position -= 1;
                if offset != 0 {
                    let new_cell = if cell_position >= offset {
                        self.cell_screen.cells[cell_position - offset]
                    } else {
                        None
                    };
                    self.cell_screen.cells[cell_position] = new_cell;
                }
            }
        }
    }
}


#[derive(Clone, PartialEq, Debug)]
enum Figure {
    Cube,
    LineHorizontal,
    LineVertical,

    LeftL0,
    LeftL90,
    LeftL180,
    LeftL270,
    RightL0,
    RightL90,
    RightL180,
    RightL270,

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

const LINE_HORIZONTAL: &'static [bool] = &[
    true, true, true, true
    ];

const LINE_VERTICAL: &'static [bool] = LINE_HORIZONTAL;

const LEFT_L_0: &'static [bool] = &[
    true,  true,
    false, true,
    false, true,
    ];

const LEFT_L_90: &'static [bool] = &[
    false, false, true,
    true,  true,  true,
    ];

const LEFT_L_180: &'static [bool] = &[
    true,  false,
    true,  false,
    true,  true,
    ];

const LEFT_L_270: &'static [bool] = &[
    true,  true,  true,
    true,  false, false,
    ];

const RIGHT_L_0: &'static [bool] = &[
    true, true,
    true, false,
    true, false,
    ];

const RIGHT_L_90: &'static [bool] = &[
    true,  true,  true,
    false, false, true,
    ];

const RIGHT_L_180: &'static [bool] = &[
    false, true,
    false, true,
    true,  true,
    ];

const RIGHT_L_270: &'static [bool] = &[
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
            &Figure::LineHorizontal => PointOffset(-2, 0),
            &Figure::LineVertical => PointOffset(0, 0),

            &Figure::LeftL0 => PointOffset(-1, 0),
            &Figure::LeftL90 => PointOffset(-2, 0),
            &Figure::LeftL180 => PointOffset(-1, 0),
            &Figure::LeftL270 => PointOffset(-2, 0),

            &Figure::RightL0 => PointOffset(-1, 0),
            &Figure::RightL90 => PointOffset(-2, 0),
            &Figure::RightL180 => PointOffset(-1, 0),
            &Figure::RightL270 => PointOffset(-2, 0),

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
            &Figure::LineHorizontal => Dimensions(4, 1),
            &Figure::LineVertical => Dimensions(1, 4),

            &Figure::LeftL0 => Dimensions(2, 3),
            &Figure::LeftL90 => Dimensions(3, 2),
            &Figure::LeftL180 => Dimensions(2, 3),
            &Figure::LeftL270 => Dimensions(3, 2),
            &Figure::RightL0 => Dimensions(2, 3),
            &Figure::RightL90 => Dimensions(3, 2),
            &Figure::RightL180 => Dimensions(2, 3),
            &Figure::RightL270 => Dimensions(3, 2),

            &Figure::LeftZigzagHorizontal => Dimensions(3, 2),
            &Figure::LeftZigzagVertical => Dimensions(2, 3),
            &Figure::RightZigzagHorizontal => Dimensions(3, 2),
            &Figure::RightZigzagVertical => Dimensions(2, 3),

            &Figure::Pyramid0 => Dimensions(3, 2),
            &Figure::Pyramid90 => Dimensions(2, 3),
            &Figure::Pyramid180 => Dimensions(3, 2),
            &Figure::Pyramid270 => Dimensions(2, 3),
        }
    }

    fn color(&self) -> TetrisCellColor {
        match self {
            &Figure::Cube => TetrisCellColor::Red,
            &Figure::LineHorizontal => TetrisCellColor::Orange,
            &Figure::LineVertical => TetrisCellColor::Orange,

            &Figure::LeftL0 => TetrisCellColor::Yellow,
            &Figure::LeftL90 => TetrisCellColor::Yellow,
            &Figure::LeftL180 => TetrisCellColor::Yellow,
            &Figure::LeftL270 => TetrisCellColor::Yellow,
            &Figure::RightL0 => TetrisCellColor::Green,
            &Figure::RightL90 => TetrisCellColor::Green,
            &Figure::RightL180 => TetrisCellColor::Green,
            &Figure::RightL270 => TetrisCellColor::Green,

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
            &Figure::LineHorizontal => LINE_HORIZONTAL,
            &Figure::LineVertical => LINE_VERTICAL,

            &Figure::LeftL0 => LEFT_L_0,
            &Figure::LeftL90 => LEFT_L_90,
            &Figure::LeftL180 => LEFT_L_180,
            &Figure::LeftL270 => LEFT_L_270,
            &Figure::RightL0 => RIGHT_L_0,
            &Figure::RightL90 => RIGHT_L_90,
            &Figure::RightL180 => RIGHT_L_180,
            &Figure::RightL270 => RIGHT_L_270,

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

    fn rotate_clockwise(self) -> (PointOffset, Self) {
        match self {
            Figure::Cube => (PointOffset(0, 0), Figure::Cube),
            Figure::LineHorizontal => (PointOffset(2, -2), Figure::LineVertical),
            Figure::LineVertical => (PointOffset(-2, 2), Figure::LineHorizontal),

            Figure::LeftL0 => (PointOffset(0, 0), Figure::LeftL90),
            Figure::LeftL90 => (PointOffset(0, 0), Figure::LeftL180),
            Figure::LeftL180 => (PointOffset(0, 0), Figure::LeftL270),
            Figure::LeftL270 => (PointOffset(0, 0), Figure::LeftL0),

            Figure::RightL0 => (PointOffset(0, 0), Figure::RightL90),
            Figure::RightL90 => (PointOffset(0, 0), Figure::RightL180),
            Figure::RightL180 => (PointOffset(0, 0), Figure::RightL270),
            Figure::RightL270 => (PointOffset(0, 0), Figure::RightL0),

            Figure::LeftZigzagHorizontal => (PointOffset(0, 0), Figure::LeftZigzagVertical),
            Figure::LeftZigzagVertical => (PointOffset(0, 0), Figure::LeftZigzagHorizontal),
            Figure::RightZigzagHorizontal => (PointOffset(0, 0), Figure::RightZigzagVertical),
            Figure::RightZigzagVertical => (PointOffset(0, 0), Figure::RightZigzagHorizontal),

            Figure::Pyramid0 => (PointOffset(0, 0), Figure::Pyramid90),
            Figure::Pyramid90 => (PointOffset(0, 0), Figure::Pyramid180),
            Figure::Pyramid180 => (PointOffset(0, 0), Figure::Pyramid270),
            Figure::Pyramid270 => (PointOffset(0, 0), Figure::Pyramid0),
        }
    }
}


impl rand::Rand for Figure {
    fn rand<R: rand::Rng>(rng: &mut R) -> Self {
        match rng.next_u32() % 28 {
            0...3 => Figure::Cube,
            4...5 => Figure::LineHorizontal,
            6...7 => Figure::LineVertical,

            8 => Figure::LeftL0,
            9 => Figure::LeftL90,
            10 => Figure::LeftL180,
            11 => Figure::LeftL270,
            12 => Figure::RightL0,
            13 => Figure::RightL90,
            14 => Figure::RightL180,
            15 => Figure::RightL270,

            16...17 => Figure::LeftZigzagHorizontal,
            18...19 => Figure::LeftZigzagVertical,
            20...21 => Figure::RightZigzagHorizontal,
            22...23 => Figure::RightZigzagVertical,

            24 => Figure::Pyramid0,
            25 => Figure::Pyramid90,
            26 => Figure::Pyramid180,
            27 => Figure::Pyramid270,

            _ => panic!("lolwut"),
        }
    }
}


fn main() {
    let sdl_context = sdl2::init().unwrap();

    let mut game = TetrisGame::new(rand::thread_rng());
    let window_size = game.window_size();

    let window = sdl_context.video().unwrap().window("Tetris", window_size.0, window_size.1).build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut renderer = window.renderer().build().unwrap();

    game.run(&mut event_pump, &mut renderer);
}
