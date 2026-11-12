mod console;
mod content;
mod content_root;
mod debug;
mod game;
mod player;
mod reachability;
mod render;
mod runtime;
mod world;

use console::{Command, SettingChange, parse_command};
use debug::DebugState;
use game::{CommandResponse, Game, GameState, apply_command};
use macroquad::prelude::*;
use player::{Input, MovementSettings};

const FIXED_STEP: f32 = 1.0 / 60.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Ashen Reach".to_owned(),
        window_width: 960,
        window_height: 600,
        ..Default::default()
    }
}

fn gameplay_input() -> Input {
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

fn execute_console(debug: &mut DebugState, session: &mut runtime::RuntimeSession) {
    let source = std::mem::take(&mut debug.console_input);
    if source.trim().is_empty() {
        return;
    }
    debug.log(format!("> {source}"));
    match parse_command(&source) {
        Err(error) => debug.log(format!("Error: {error}")),
        Ok(Command::ReloadContent) => match session.reload("prototype_dungeon") {
            Ok(()) => debug.log("Content reloaded; prototype dungeon restarted"),
            Err(errors) => {
                for error in errors {
                    debug.log(format!("Content error: {error}"));
                }
            }
        },
        Ok(command) => match apply_command(&mut session.game, command) {
            Err(error) => debug.log(format!("Error: {error}")),
            Ok(CommandResponse::Message(message)) => debug.log(message),
            Ok(CommandResponse::Setting(SettingChange::Speed(value))) => {
                debug.speed_multiplier = value;
                debug.log(format!("Speed multiplier set to {value:.2}"));
            }
            Ok(CommandResponse::Setting(SettingChange::Fov(value))) => {
                debug.fov_degrees = value;
                debug.log(format!("FOV set to {value:.1} degrees"));
            }
        },
    }
}

fn console_input(debug: &mut DebugState) -> bool {
    if is_key_pressed(KeyCode::GraveAccent) {
        debug.console_open = !debug.console_open;
        debug.console_input.clear();
        debug.log(if debug.console_open {
            "Console opened"
        } else {
            "Console closed"
        });
        return true;
    }
    if !debug.console_open {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        debug.console_open = false;
        debug.console_input.clear();
        return true;
    }
    if is_key_pressed(KeyCode::Backspace) {
        debug.console_input.pop();
    }
    if is_key_pressed(KeyCode::Enter) {
        return true;
    }
    while let Some(character) = get_char_pressed() {
        if !character.is_control() && character != '`' {
            debug.console_input.push(character);
        }
    }
    false
}

#[macroquad::main(window_conf)]
async fn main() {
    let startup_config = content_root::StartupConfig::default();
    let content_root = startup_config
        .resolve_content_root()
        .unwrap_or_else(|error| panic!("Content root resolution failed: {error}"));
    let mut session = runtime::RuntimeSession::load(&content_root, "prototype_dungeon")
        .unwrap_or_else(|errors| panic!("Content validation failed:\n{}", errors.join("\n")));
    let mut debug = DebugState::new();
    let mut accumulator = 0.0;
    let mut pending_interaction = false;

    loop {
        let console_was_open = debug.console_open;
        let console_consumed_frame = console_input(&mut debug) || console_was_open;
        if console_consumed_frame && debug.console_open && is_key_pressed(KeyCode::Enter) {
            execute_console(&mut debug, &mut session);
            accumulator = 0.0;
        }
        if !debug.console_open && !console_consumed_frame {
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
            if session.game.state == GameState::Completed && is_key_pressed(KeyCode::R) {
                if let Err(error) = session.restart("prototype_dungeon") {
                    debug.log(format!("Restart failed: {error}"));
                }
                accumulator = 0.0;
            }
            if is_key_pressed(KeyCode::F3) {
                debug.overlay_visible = !debug.overlay_visible;
            }
            if is_key_pressed(KeyCode::F4) {
                debug.noclip = !debug.noclip;
                debug.log(format!(
                    "Noclip {}",
                    if debug.noclip { "enabled" } else { "disabled" }
                ));
            }
            if is_key_pressed(KeyCode::F5) {
                debug.fog = !debug.fog;
            }
            if is_key_pressed(KeyCode::F6) {
                debug.minimap = !debug.minimap;
            }
            if is_key_pressed(KeyCode::F7) {
                debug.show_rays = !debug.show_rays;
                if debug.show_rays {
                    debug.minimap = true;
                }
            }
        }

        let frame_time = get_frame_time();
        let mut fixed_steps = 0;
        if !debug.console_open
            && !console_consumed_frame
            && session.game.state == GameState::Playing
        {
            let mut input = gameplay_input();
            pending_interaction |= input.interact;
            input.interact = false;
            accumulator += frame_time.min(0.25);
            while accumulator >= FIXED_STEP {
                input.interact = pending_interaction;
                session.game.update(
                    input,
                    MovementSettings {
                        noclip: debug.noclip,
                        speed_multiplier: debug.speed_multiplier,
                    },
                    FIXED_STEP,
                );
                pending_interaction = false;
                input.interact = false;
                accumulator -= FIXED_STEP;
                fixed_steps += 1;
            }
        } else {
            accumulator = 0.0;
            pending_interaction = false;
        }

        clear_background(BLACK);
        if session.game.state == GameState::Completed {
            draw_completion(&session.game);
        } else {
            render::draw_world(
                &session.game.world,
                &session.game.player,
                debug.fov_degrees,
                debug.fog,
            );
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
                    session.game.player.x, session.game.player.y, session.game.player.steps_taken
                ),
                18.0,
                screen_height() - 18.0,
                18.0,
                LIGHTGRAY,
            );
        }
        if debug.overlay_visible || debug.minimap || debug.console_open {
            debug::draw_debug(&debug, &session.game, frame_time, fixed_steps);
        }
        next_frame().await;
    }
}

fn draw_completion(game: &Game) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(13, 16, 22, 255),
    );
    draw_text("LEVEL COMPLETE", 300.0, 230.0, 42.0, GOLD);
    draw_text(
        format!("Time: {:.1} seconds", game.elapsed_seconds),
        340.0,
        290.0,
        26.0,
        WHITE,
    );
    draw_text(
        format!("Steps taken: {}", game.player.steps_taken),
        360.0,
        330.0,
        26.0,
        WHITE,
    );
    draw_text(
        "Press R to restart or ` for the console",
        300.0,
        390.0,
        22.0,
        LIGHTGRAY,
    );
}

#[cfg(test)]
mod tests {
    use crate::debug::DebugState;

    #[test]
    fn debug_state_starts_hidden_with_default_overrides() {
        let state = DebugState::new();
        assert!(!state.overlay_visible);
        assert!(!state.noclip);
        assert!(state.fog);
        assert_eq!(state.speed_multiplier, 1.0);
        assert_eq!(state.fov_degrees, 60.0);
    }
}
