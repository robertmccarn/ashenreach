mod game;
mod player;
mod render;
mod world;

use game::{Game, GameState, target_tile};
use macroquad::prelude::*;
use player::{Input, Player};
use world::{Tile, World};

const FIXED_STEP: f32 = 1.0 / 60.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Ashen Reach".to_owned(),
        window_width: 960,
        window_height: 600,
        ..Default::default()
    }
}

fn new_level() -> (World, Player, Game) {
    let world = World::prototype_dungeon();
    let (x, y) = world.start_position();
    (world, Player::new(x, y), Game::new())
}

fn input_state() -> Input {
    Input {
        forward: is_key_down(KeyCode::W) || is_key_down(KeyCode::Up),
        backward: is_key_down(KeyCode::S) || is_key_down(KeyCode::Down),
        turn_left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::Q) || is_key_down(KeyCode::A),
        turn_right: is_key_down(KeyCode::Right)
            || is_key_down(KeyCode::E)
            || is_key_down(KeyCode::D),
        interact: is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::F),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let (mut world, mut player, mut game) = new_level();
    let mut accumulator = 0.0;
    let mut pending_interaction = false;

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if game.state == GameState::Completed {
            if is_key_pressed(KeyCode::R) {
                (world, player, game) = new_level();
                accumulator = 0.0;
                pending_interaction = false;
            }
            clear_background(Color::from_rgba(13, 16, 22, 255));
            draw_text("LEVEL COMPLETE", 300.0, 230.0, 42.0, GOLD);
            draw_text(
                format!("Time: {:.1} seconds", game.elapsed_seconds),
                340.0,
                290.0,
                26.0,
                WHITE,
            );
            draw_text(
                format!("Steps taken: {}", player.steps_taken),
                360.0,
                330.0,
                26.0,
                WHITE,
            );
            draw_text("Press R to play again", 365.0, 400.0, 22.0, LIGHTGRAY);
            next_frame().await;
            continue;
        }

        let mut frame_input = input_state();
        pending_interaction |= frame_input.interact;
        accumulator += get_frame_time().min(0.25);
        while accumulator >= FIXED_STEP {
            frame_input.interact = pending_interaction;
            game.update(&mut world, &mut player, frame_input, FIXED_STEP);
            pending_interaction = false;
            frame_input.interact = false;
            accumulator -= FIXED_STEP;
        }

        clear_background(BLACK);
        render::draw_world(&world, &player);
        draw_text(
            "ASHEN REACH  |  WASD move  |  arrows / Q-E turn",
            18.0,
            28.0,
            22.0,
            WHITE,
        );
        draw_text(
            format!(
                "Position {:.1}, {:.1}   Steps {}",
                player.x, player.y, player.steps_taken
            ),
            18.0,
            screen_height() - 18.0,
            18.0,
            LIGHTGRAY,
        );
        if let Some((x, y)) = target_tile(&world, &player) {
            let prompt = match world.tile_at(x, y) {
                Tile::Door { open: true } => "Press F or Space to close",
                Tile::Door { open: false } => "Press F or Space to open",
                Tile::Exit => "Press F or Space to exit",
                Tile::Floor | Tile::Wall => "",
            };
            draw_text(
                prompt,
                screen_width() * 0.5 - 130.0,
                screen_height() * 0.78,
                24.0,
                GOLD,
            );
        }
        next_frame().await;
    }
}
