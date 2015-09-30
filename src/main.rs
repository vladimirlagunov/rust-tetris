extern crate sdl2;
extern crate sdl2_sys;
extern crate rand;


use sdl2::pixels::Color;
use sdl2::surface::Surface;
use sdl2::video::{Window};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2_sys::event::SDL_USEREVENT;

use rand::random;


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

    let timer = timer_subsystem.add_timer(3000, Box::new(|| {
        event_subsystem.push_event(Event::User {
            code: 0,
            window_id: window_id,
            timestamp: 0,
            type_: SDL_USEREVENT,
        }).unwrap();
        println!("Pushed user event");
        3000
    }));

    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown {keycode: Some(Keycode::Q), ..} => 
                {
                    running = false;
                },
                Event::KeyDown {keycode: Some(..), ..} | Event::User {code: 0, ..} => {
                    renderer.set_draw_color(Color::RGB(random(), random(), random()));
                    renderer.clear();
                    renderer.present();
                },
                e => {
                    println!("{:?}", e);
                },
            }
        }
    }

    drop(timer);
}
