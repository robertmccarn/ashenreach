use macroquad::prelude::*;

use crate::{player::Player, world::World};

const FOV: f32 = 1.05;
const RAY_COUNT: usize = 240;

fn cast_ray(world: &World, player: &Player, angle: f32) -> (f32, bool, crate::world::Tile) {
    let (dx, dy) = (angle.cos(), angle.sin());
    let mut distance = 0.0;
    while distance < 20.0 {
        distance += 0.025;
        let x = player.x + dx * distance;
        let y = player.y + dy * distance;
        let tile = world.tile_at_position(x, y);
        if matches!(tile, crate::world::Tile::Exit)
            || matches!(
                tile,
                crate::world::Tile::Door { open: false } | crate::world::Tile::Wall
            )
        {
            let fx = x.fract();
            let fy = y.fract();
            return (distance, fx.min(1.0 - fx) < fy.min(1.0 - fy), tile);
        }
    }
    (20.0, false, crate::world::Tile::Wall)
}

pub fn draw_world(world: &World, player: &Player) {
    let width = screen_width();
    let height = screen_height();
    draw_rectangle(
        0.0,
        0.0,
        width,
        height * 0.5,
        Color::from_rgba(27, 35, 49, 255),
    );
    draw_rectangle(
        0.0,
        height * 0.5,
        width,
        height * 0.5,
        Color::from_rgba(57, 49, 39, 255),
    );
    let strip_width = width / RAY_COUNT as f32;
    for ray in 0..RAY_COUNT {
        let offset = ray as f32 / RAY_COUNT as f32 - 0.5;
        let angle_offset = offset * FOV;
        let (distance, side, tile) = cast_ray(world, player, player.angle + angle_offset);
        let corrected_distance = (distance * angle_offset.cos()).max(0.05);
        let wall_height = (height / corrected_distance).min(height * 2.0);
        let shade = (220.0 / (1.0 + corrected_distance * 0.32)) as u8;
        let shade = if side {
            shade.saturating_sub(38)
        } else {
            shade
        };
        let wall_color = match tile {
            crate::world::Tile::Door { open: false } => Color::from_rgba(shade, shade / 2, 24, 255),
            crate::world::Tile::Exit => Color::from_rgba(20, shade, shade / 3, 255),
            _ => Color::from_rgba(
                shade,
                shade.saturating_sub(24),
                shade.saturating_sub(50),
                255,
            ),
        };
        draw_rectangle(
            ray as f32 * strip_width,
            (height - wall_height) * 0.5,
            strip_width + 1.0,
            wall_height,
            wall_color,
        );
    }
}
