use crate::world::World;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub forward: bool,
    pub backward: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub interact: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementSettings {
    pub noclip: bool,
    pub speed_multiplier: f32,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            noclip: false,
            speed_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub steps_taken: u32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            angle: 0.0,
            steps_taken: 0,
        }
    }

    pub fn update(&mut self, world: &World, input: Input, settings: MovementSettings, dt: f32) {
        let turn = input.turn_right as i8 - input.turn_left as i8;
        self.angle += turn as f32 * 1.8 * dt;

        let movement = input.forward as i8 - input.backward as i8;
        let distance = movement as f32 * 2.2 * settings.speed_multiplier * dt;
        let previous_cell = (self.x.floor() as i32, self.y.floor() as i32);
        let next_x = self.x + self.angle.cos() * distance;
        let next_y = self.y + self.angle.sin() * distance;
        if settings.noclip || !world.is_blocking(next_x, self.y) {
            self.x = next_x;
        }
        if settings.noclip || !world.is_blocking(self.x, next_y) {
            self.y = next_y;
        }
        let current_cell = (self.x.floor() as i32, self.y.floor() as i32);
        if current_cell != previous_cell {
            self.steps_taken = self.steps_taken.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Input, MovementSettings, Player};
    use crate::world::World;

    #[test]
    fn movement_respects_walls_and_door_state() {
        let mut world = World::prototype_dungeon();
        let mut player = Player::new(4.5, 1.5);
        let forward = Input {
            forward: true,
            ..Input::default()
        };
        for _ in 0..60 {
            player.update(&world, forward, MovementSettings::default(), 1.0 / 60.0);
        }
        assert!(player.x < 5.0);
        assert!(world.is_blocking(5.5, 1.5));

        assert!(world.toggle_door(5, 1));
        assert!(!world.is_blocking(5.5, 1.5));
        for _ in 0..30 {
            player.update(&world, forward, MovementSettings::default(), 1.0 / 60.0);
        }
        assert!(player.x > 5.0);

        player.x = 3.5;
        player.y = 2.5;
        player.angle = 0.0;
        for _ in 0..60 {
            player.update(&world, forward, MovementSettings::default(), 1.0 / 60.0);
        }
        assert!(player.x < 4.0, "player crossed the wall at (4, 2)");
    }
}
