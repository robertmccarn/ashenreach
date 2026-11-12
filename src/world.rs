use crate::reachability::flood_fill;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Door { open: bool },
    Exit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    start: (usize, usize),
}

impl World {
    pub fn prototype_dungeon() -> Self {
        let map = serde_json::from_str(include_str!("../content/maps/prototype_dungeon.json"))
            .expect("embedded prototype map must be valid JSON");
        Self::from_map(&map).expect("embedded prototype map must be valid")
    }

    pub fn from_map(map: &crate::content::MapDefinition) -> Result<Self, String> {
        if map.rows.is_empty() {
            return Err(format!("map `{}` has no rows", map.id));
        }
        let width = map.rows[0].len();
        if width == 0 {
            return Err(format!("map `{}` has an empty row", map.id));
        }
        let mut start = (1, 1);
        let mut start_count = 0;
        let mut tiles = Vec::with_capacity(width * map.rows.len());
        for (y, row) in map.rows.iter().enumerate() {
            if row.len() != width {
                return Err(format!("map `{}` row {y} has inconsistent width", map.id));
            }
            for (x, cell) in row.bytes().enumerate() {
                tiles.push(match cell {
                    b'#' => Tile::Wall,
                    b'D' => Tile::Door { open: false },
                    b'X' => Tile::Exit,
                    b'S' => {
                        start = (x, y);
                        start_count += 1;
                        Tile::Floor
                    }
                    b'.' => Tile::Floor,
                    _ => {
                        return Err(format!(
                            "map `{}` row {y} has invalid tile `{}`",
                            map.id, cell as char
                        ));
                    }
                });
            }
        }
        if start_count != 1 {
            return Err(format!("map `{}` must contain exactly one start", map.id));
        }
        let world = Self {
            width,
            height: map.rows.len(),
            tiles,
            start,
        };
        if !world.reachable_exit_from_start() {
            return Err(format!(
                "map `{}` must have at least one exit and every exit must be reachable from its start",
                map.id
            ));
        }
        Ok(world)
    }

    pub fn start_position(&self) -> (f32, f32) {
        (self.start.0 as f32 + 0.5, self.start.1 as f32 + 0.5)
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn tile_at(&self, x: isize, y: isize) -> Tile {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return Tile::Wall;
        }
        self.tiles[y as usize * self.width + x as usize]
    }

    pub fn tile_at_position(&self, x: f32, y: f32) -> Tile {
        self.tile_at(x.floor() as isize, y.floor() as isize)
    }

    pub fn is_blocking(&self, x: f32, y: f32) -> bool {
        matches!(
            self.tile_at_position(x, y),
            Tile::Wall | Tile::Door { open: false }
        )
    }

    pub fn toggle_door(&mut self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return false;
        }
        let tile = &mut self.tiles[y as usize * self.width + x as usize];
        if let Tile::Door { open } = tile {
            *open = !*open;
            true
        } else {
            false
        }
    }

    pub fn set_all_doors(&mut self, open: bool) {
        for tile in &mut self.tiles {
            if let Tile::Door { open: state } = tile {
                *state = open;
            }
        }
    }

    pub fn reachable_exit_from_start(&self) -> bool {
        let reachable = flood_fill(self.width, self.height, self.start, |x, y| {
            !matches!(self.tiles[y * self.width + x], Tile::Wall)
        })
        .expect("a loaded world must have valid dimensions");
        self.tiles.contains(&Tile::Exit)
            && self
                .tiles
                .iter()
                .zip(reachable)
                .all(|(tile, is_reachable)| *tile != Tile::Exit || is_reachable)
    }

    #[cfg(test)]
    pub fn all_doors_open(&self) -> bool {
        self.tiles
            .iter()
            .all(|tile| !matches!(tile, Tile::Door { open: false }))
    }

    #[cfg(test)]
    pub fn all_doors_closed(&self) -> bool {
        self.tiles
            .iter()
            .all(|tile| !matches!(tile, Tile::Door { open: true }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Tile, World};

    #[test]
    fn flood_fill_from_start_reaches_exit_through_doors() {
        let world = World::prototype_dungeon();
        assert!(world.reachable_exit_from_start());
        assert_eq!(world.tile_at(13, 1), Tile::Exit);
        assert_eq!(world.tile_at(5, 1), Tile::Door { open: false });
        assert_eq!(world.tile_at(9, 1), Tile::Door { open: false });
    }

    #[test]
    fn world_rejects_a_map_with_an_unreachable_exit() {
        let map = crate::content::MapDefinition {
            id: "disconnected".to_owned(),
            name: "Disconnected".to_owned(),
            rows: vec!["S#X".to_owned()],
        };

        let error = World::from_map(&map).unwrap_err();
        assert!(error.contains("disconnected"));
        assert!(error.contains("every exit must be reachable"));
    }
}
