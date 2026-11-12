use std::collections::VecDeque;

use macroquad::prelude::*;

use crate::{
    game::Game,
    player::Player,
    world::{Tile, World},
};

pub struct DebugState {
    pub overlay_visible: bool,
    pub noclip: bool,
    pub fog: bool,
    pub minimap: bool,
    pub show_rays: bool,
    pub console_open: bool,
    pub console_input: String,
    pub console_log: VecDeque<String>,
    pub speed_multiplier: f32,
    pub fov_degrees: f32,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            overlay_visible: false,
            noclip: false,
            fog: true,
            minimap: false,
            show_rays: false,
            console_open: false,
            console_input: String::new(),
            console_log: VecDeque::new(),
            speed_multiplier: 1.0,
            fov_degrees: 60.0,
        }
    }

    pub fn log(&mut self, line: impl Into<String>) {
        self.console_log.push_back(line.into());
        while self.console_log.len() > 10 {
            self.console_log.pop_front();
        }
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn draw_debug(debug: &DebugState, game: &Game, frame_time: f32, fixed_steps: usize) {
    if debug.overlay_visible {
        draw_stats(debug, game, frame_time, fixed_steps);
    }
    if debug.minimap {
        draw_minimap(debug, &game.world, &game.player);
    }
    if debug.console_open || !debug.console_log.is_empty() && debug.overlay_visible {
        draw_console(debug);
    }
}

fn draw_stats(debug: &DebugState, game: &Game, frame_time: f32, fixed_steps: usize) {
    let player = &game.player;
    let tile_x = player.x.floor() as isize;
    let tile_y = player.y.floor() as isize;
    let facing = faced_cell(&game.world, player);
    let faced = facing
        .map(|(x, y)| format!("{x}, {y}: {}", tile_name(game.world.tile_at(x, y))))
        .unwrap_or_else(|| "none".to_owned());
    let lines = [
        format!(
            "FPS: {:.0}   Frame: {:.2} ms   Fixed steps: {fixed_steps}",
            1.0 / frame_time.max(0.000_01),
            frame_time * 1000.0
        ),
        format!(
            "Position: {:.2}, {:.2}   Tile: {tile_x}, {tile_y}",
            player.x, player.y
        ),
        format!(
            "Facing: {:.1} deg   Tile ahead: {faced}",
            player.angle.to_degrees()
        ),
        format!(
            "State: {:?}   Time: {:.2}s   Steps: {}",
            game.state, game.elapsed_seconds, player.steps_taken
        ),
        format!(
            "F3 overlay [{}]  F4 noclip [{}]  F5 fog [{}]",
            on(debug.overlay_visible),
            on(debug.noclip),
            on(debug.fog)
        ),
        format!(
            "F6 minimap [{}]  F7 rays [{}]  {} rays  speed {:.2}  FOV {:.1}",
            on(debug.minimap),
            on(debug.show_rays),
            crate::render::RAY_COUNT,
            debug.speed_multiplier,
            debug.fov_degrees
        ),
    ];
    draw_rectangle(10.0, 10.0, 500.0, 150.0, Color::from_rgba(0, 0, 0, 190));
    for (index, line) in lines.iter().enumerate() {
        draw_text(line, 20.0, 32.0 + index as f32 * 21.0, 18.0, LIME);
    }
}

fn draw_minimap(debug: &DebugState, world: &World, player: &Player) {
    let (width, height) = world.dimensions();
    let cell = (240.0 / width.max(height) as f32).min(20.0);
    let left = screen_width() - width as f32 * cell - 18.0;
    let top = 18.0;
    draw_rectangle(
        left - 5.0,
        top - 5.0,
        width as f32 * cell + 10.0,
        height as f32 * cell + 10.0,
        Color::from_rgba(0, 0, 0, 190),
    );
    for y in 0..height {
        for x in 0..width {
            let color = match world.tile_at(x as isize, y as isize) {
                Tile::Wall => Color::from_rgba(105, 108, 116, 255),
                Tile::Floor => Color::from_rgba(35, 38, 43, 255),
                Tile::Door { open: false } => Color::from_rgba(191, 91, 35, 255),
                Tile::Door { open: true } => Color::from_rgba(116, 82, 57, 255),
                Tile::Exit => Color::from_rgba(25, 190, 85, 255),
            };
            draw_rectangle(
                left + x as f32 * cell,
                top + y as f32 * cell,
                cell - 1.0,
                cell - 1.0,
                color,
            );
        }
    }
    let px = left + player.x * cell;
    let py = top + player.y * cell;
    draw_circle(px, py, cell * 0.28, YELLOW);
    draw_line(
        px,
        py,
        px + player.angle.cos() * cell,
        py + player.angle.sin() * cell,
        2.0,
        YELLOW,
    );
    if debug.show_rays {
        const VISUAL_RAYS: usize = 24;
        for ray in 0..VISUAL_RAYS {
            let offset =
                (ray as f32 / (VISUAL_RAYS - 1) as f32 - 0.5) * debug.fov_degrees.to_radians();
            let angle = player.angle + offset;
            let mut distance = 0.0;
            while distance < 12.0 {
                distance += 0.05;
                let x = player.x + angle.cos() * distance;
                let y = player.y + angle.sin() * distance;
                if matches!(
                    world.tile_at_position(x, y),
                    Tile::Wall | Tile::Door { open: false } | Tile::Exit
                ) {
                    break;
                }
            }
            draw_line(
                px,
                py,
                px + angle.cos() * distance * cell,
                py + angle.sin() * distance * cell,
                1.0,
                Color::from_rgba(255, 235, 100, 95),
            );
        }
    }
}

fn draw_console(debug: &DebugState) {
    let bottom = screen_height() - 32.0;
    draw_rectangle(
        10.0,
        bottom - 220.0,
        screen_width() - 20.0,
        240.0,
        Color::from_rgba(0, 0, 0, 210),
    );
    let start_y = bottom - 205.0;
    for (index, line) in debug.console_log.iter().enumerate() {
        draw_text(line, 20.0, start_y + index as f32 * 19.0, 17.0, LIGHTGRAY);
    }
    let marker = if debug.console_open { "` >" } else { "" };
    draw_text(
        format!("{marker} {}", debug.console_input),
        20.0,
        bottom,
        20.0,
        YELLOW,
    );
}

fn faced_cell(_world: &World, player: &Player) -> Option<(isize, isize)> {
    let x = player.x + player.angle.cos() * 0.75;
    let y = player.y + player.angle.sin() * 0.75;
    Some((x.floor() as isize, y.floor() as isize))
}

fn tile_name(tile: Tile) -> &'static str {
    match tile {
        Tile::Floor => "floor",
        Tile::Wall => "wall",
        Tile::Door { open: false } => "closed door",
        Tile::Door { open: true } => "open door",
        Tile::Exit => "exit",
    }
}

fn on(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}
