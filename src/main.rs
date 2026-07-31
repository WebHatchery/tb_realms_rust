//! Macroquad game template wired to macroquad-toolkit.

use macroquad::prelude::*;
use macroquad_toolkit::capture;

mod data;
mod game;
mod market;
mod state;
mod ui;
mod world;

use game::Game;

fn window_conf() -> Conf {
    capture::capture_window_conf(
        "TB_REALMS",
        "Tradeborn Realms",
        ui::LOGICAL_WIDTH as i32,
        ui::LOGICAL_HEIGHT as i32,
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new().await;

    // Screenshot harness: when TB_REALMS_CAPTURE_PATH is set, render
    // deterministic frames, write a PNG, and exit. The scene is applied once
    // the world has answered, because every screen here is downstream of that.
    if let Some(config) = capture::CaptureConfig::from_env("TB_REALMS") {
        game.begin_capture_scene(&config.scene);
        capture::run_capture(&config, |dt| {
            game.update(dt);
            game.draw();
        })
        .await;
        return;
    }

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
