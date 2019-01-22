#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate clamp;
#[macro_use]
extern crate conrod_core;
extern crate conrod_glium;
extern crate conrod_winit;
extern crate image;
extern crate nd_iter;
extern crate rand;
extern crate rand_xorshift;

use std::time::{Duration, SystemTime};

mod client;
mod input;
mod physics;
mod render;

use client::Client;

// Get the time since `prev_time` in seconds
fn get_time_delta(prev_time: &SystemTime) -> f32 {
    let elapsed = prev_time.elapsed().unwrap_or_else(|_| Duration::new(0, 0));
    elapsed.as_secs() as f32 + elapsed.subsec_millis() as f32 / 1000.0
}

fn main() {
    let mut client = Client::init();

    // Time of the previous frame
    let mut prev_time = SystemTime::now();
    // Gameloop
    while client.state.running {
        let dt = get_time_delta(&prev_time);
        prev_time = SystemTime::now();
        input::do_input(&mut client);
        client::update(&mut client, dt);
        render::render(&mut client.gfx, &mut client.state);
        client.state.frame += 1;
    }
}
