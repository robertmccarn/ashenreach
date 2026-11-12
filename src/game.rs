use crate::{
    player::{Input, Player},
    world::{Tile, World},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Completed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub state: GameState,
    pub elapsed_seconds: f32,
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: GameState::Playing,
            elapsed_seconds: 0.0,
        }
    }

    pub fn update(&mut self, world: &mut World, player: &mut Player, input: Input, dt: f32) {
        if self.state == GameState::Completed {
            return;
        }
        self.elapsed_seconds += dt;
        player.update(world, input, dt);
        if input.interact
            && let Some((x, y)) = target_tile(world, player)
        {
            match world.tile_at(x, y) {
                Tile::Door { .. } => {
                    world.toggle_door(x, y);
                }
                Tile::Exit => self.state = GameState::Completed,
                Tile::Floor | Tile::Wall => {}
            }
        }
    }
}

pub fn target_tile(world: &World, player: &Player) -> Option<(isize, isize)> {
    let mut distance = 0.25;
    while distance <= 1.5 {
        let x = player.x + player.angle.cos() * distance;
        let y = player.y + player.angle.sin() * distance;
        let cell = (x.floor() as isize, y.floor() as isize);
        match world.tile_at(cell.0, cell.1) {
            Tile::Door { .. } | Tile::Exit => return Some(cell),
            Tile::Wall => return None,
            Tile::Floor => distance += 0.05,
        }
    }
    None
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Game, GameState};
    use crate::{
        player::{Input, Player},
        world::World,
    };

    #[test]
    fn interaction_toggles_a_door_and_completes_at_exit() {
        let mut world = World::prototype_dungeon();
        let mut player = Player::new(4.5, 1.5);
        let interact = Input {
            interact: true,
            ..Input::default()
        };
        let mut game = Game::new();
        game.update(&mut world, &mut player, interact, 1.0 / 60.0);
        assert!(!world.is_blocking(5.5, 1.5));

        player.x = 12.5;
        player.y = 1.5;
        player.angle = 0.0;
        game.update(&mut world, &mut player, interact, 1.0 / 60.0);
        assert_eq!(game.state, GameState::Completed);
    }

    #[test]
    fn identical_fixed_step_inputs_produce_identical_results() {
        let inputs = [
            Input {
                forward: true,
                ..Input::default()
            },
            Input {
                turn_right: true,
                ..Input::default()
            },
            Input {
                forward: true,
                ..Input::default()
            },
            Input::default(),
        ];
        let simulate = || {
            let mut world = World::prototype_dungeon();
            let (x, y) = world.start_position();
            let mut player = Player::new(x, y);
            let mut game = Game::new();
            for input in inputs {
                for _ in 0..10 {
                    game.update(&mut world, &mut player, input, 1.0 / 60.0);
                }
            }
            (player, game.state, game.elapsed_seconds)
        };
        assert_eq!(simulate(), simulate());
    }
}
