use crate::{
    console::{Command, SettingChange},
    player::{Input, MovementSettings, Player},
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
    pub world: World,
    pub player: Player,
}

impl Game {
    pub fn new() -> Self {
        let world = World::prototype_dungeon();
        Self::from_world(world)
    }

    pub fn from_map(map: &crate::content::MapDefinition) -> Result<Self, String> {
        Ok(Self::from_world(World::from_map(map)?))
    }

    fn from_world(world: World) -> Self {
        let (x, y) = world.start_position();
        Self {
            state: GameState::Playing,
            elapsed_seconds: 0.0,
            world,
            player: Player::new(x, y),
        }
    }

    pub fn update(&mut self, input: Input, settings: MovementSettings, dt: f32) {
        if self.state == GameState::Completed {
            return;
        }
        self.elapsed_seconds += dt;
        self.player.update(&self.world, input, settings, dt);
        if input.interact
            && let Some((x, y)) = target_tile(&self.world, &self.player)
        {
            match self.world.tile_at(x, y) {
                Tile::Door { .. } => {
                    self.world.toggle_door(x, y);
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

pub fn apply_command(game: &mut Game, command: Command) -> Result<CommandResponse, String> {
    match command {
        Command::Teleport(x, y) => {
            if !x.is_finite() || !y.is_finite() {
                return Err("tp coordinates must be finite numbers".to_owned());
            }
            let (width, height) = game.world.dimensions();
            let (tile_x, tile_y) = (x.floor() as isize, y.floor() as isize);
            if tile_x < 0 || tile_y < 0 || tile_x >= width as isize || tile_y >= height as isize {
                return Err("tp target is outside the map".to_owned());
            }
            if game.world.is_blocking(x.floor() + 0.5, y.floor() + 0.5) {
                return Err("tp target is a wall or closed door".to_owned());
            }
            game.player.x = x.floor() + 0.5;
            game.player.y = y.floor() + 0.5;
            Ok(CommandResponse::Message(format!(
                "Teleported to tile {tile_x}, {tile_y}"
            )))
        }
        Command::Speed(multiplier) => {
            Ok(CommandResponse::Setting(SettingChange::Speed(multiplier)))
        }
        Command::Fov(degrees) => Ok(CommandResponse::Setting(SettingChange::Fov(degrees))),
        Command::OpenAll => {
            game.world.set_all_doors(true);
            Ok(CommandResponse::Message("Opened every door".to_owned()))
        }
        Command::CloseAll => {
            game.world.set_all_doors(false);
            Ok(CommandResponse::Message("Closed every door".to_owned()))
        }
        Command::Win => {
            game.state = GameState::Completed;
            Ok(CommandResponse::Message("Level completed".to_owned()))
        }
        Command::Restart => {
            *game = Game::new();
            Ok(CommandResponse::Message("Level restarted".to_owned()))
        }
        Command::ReloadContent => Err("reload_content must be handled by the content service".to_owned()),
        Command::Help => Ok(CommandResponse::Message(
            "Commands: tp <x> <y>, speed <n>, fov <deg>, open_all, close_all, win, restart, reload_content, help"
                .to_owned(),
        )),
    }
}

#[derive(Debug)]
pub enum CommandResponse {
    Message(String),
    Setting(SettingChange),
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandResponse, Game, GameState, apply_command};
    use crate::{
        console::{Command, SettingChange},
        player::{Input, MovementSettings},
    };

    #[test]
    fn teleport_rejects_out_of_bounds_and_wall_tiles() {
        let mut game = Game::new();
        assert!(
            apply_command(&mut game, Command::Teleport(-1.0, 0.0))
                .unwrap_err()
                .contains("outside")
        );
        assert!(
            apply_command(&mut game, Command::Teleport(0.0, 0.0))
                .unwrap_err()
                .contains("wall")
        );
    }

    #[test]
    fn open_all_and_close_all_update_every_door() {
        let mut game = Game::new();
        apply_command(&mut game, Command::OpenAll).unwrap();
        assert!(game.world.all_doors_open());
        apply_command(&mut game, Command::CloseAll).unwrap();
        assert!(game.world.all_doors_closed());
    }

    #[test]
    fn noclip_allows_movement_through_walls() {
        let mut game = Game::new();
        game.player.x = 3.5;
        game.player.y = 2.5;
        game.player.angle = std::f32::consts::PI;
        let input = Input {
            forward: true,
            ..Input::default()
        };
        let settings = MovementSettings {
            noclip: true,
            speed_multiplier: 1.0,
        };
        for _ in 0..60 {
            game.update(input, settings, 1.0 / 60.0);
        }
        assert!(game.player.x < 3.0);
    }

    #[test]
    fn fixed_step_input_sequence_remains_deterministic() {
        let sequence = [
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
            let mut game = Game::new();
            for input in sequence {
                for _ in 0..10 {
                    game.update(input, MovementSettings::default(), 1.0 / 60.0);
                }
            }
            game
        };
        let first = simulate();
        let second = simulate();
        assert_eq!(first.player, second.player);
        assert_eq!(first.state, second.state);
    }

    #[test]
    fn interaction_toggles_a_door_and_completes_at_exit() {
        let mut game = Game::new();
        game.player.x = 4.5;
        game.player.y = 1.5;
        let interact = Input {
            interact: true,
            ..Input::default()
        };
        game.update(interact, MovementSettings::default(), 1.0 / 60.0);
        assert!(!game.world.is_blocking(5.5, 1.5));

        game.player.x = 12.5;
        game.player.y = 1.5;
        game.player.angle = 0.0;
        game.update(interact, MovementSettings::default(), 1.0 / 60.0);
        assert_eq!(game.state, GameState::Completed);
    }

    #[test]
    fn command_settings_are_returned_for_debug_state_to_apply() {
        let mut game = Game::new();
        assert!(matches!(
            apply_command(&mut game, Command::Speed(2.0)).unwrap(),
            CommandResponse::Setting(SettingChange::Speed(2.0))
        ));
        assert!(matches!(
            apply_command(&mut game, Command::Fov(90.0)).unwrap(),
            CommandResponse::Setting(SettingChange::Fov(90.0))
        ));
    }

    #[test]
    fn win_command_completes_the_level() {
        let mut game = Game::new();
        apply_command(&mut game, Command::Win).unwrap();
        assert_eq!(game.state, GameState::Completed);
    }
}
