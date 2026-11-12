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
        let height = map.rows.len();
        let tile_count = width.checked_mul(height).ok_or_else(|| {
            format!(
                "map `{}` dimensions {width}x{height} exceed the supported tile capacity",
                map.id
            )
        })?;
        let mut start = (1, 1);
        let mut start_count = 0;
        let mut exits = Vec::new();
        let mut tiles = Vec::with_capacity(tile_count);
        for (y, row) in map.rows.iter().enumerate() {
            if row.is_empty() {
                return Err(format!("map `{}` row {y} is empty", map.id));
            }
            if row.len() != width {
                return Err(format!("map `{}` row {y} has inconsistent width", map.id));
            }
            for (x, cell) in row.bytes().enumerate() {
                tiles.push(match cell {
                    b'#' => Tile::Wall,
                    b'D' => Tile::Door { open: false },
                    b'X' => {
                        exits.push((x, y));
                        Tile::Exit
                    }
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
            return Err(format!(
                "map `{}` must contain exactly one start; found {start_count}",
                map.id
            ));
        }
        if exits.is_empty() {
            return Err(format!("map `{}` must contain at least one exit", map.id));
        }
        let world = Self {
            width,
            height,
            tiles,
            start,
        };
        let reachable = flood_fill(width, height, start, |x, y| {
            !matches!(world.tiles[y * width + x], Tile::Wall)
        })
        .expect("checked map dimensions must produce a valid grid");
        for (x, y) in exits {
            if !reachable[y * width + x] {
                return Err(format!(
                    "map `{}` exit at ({x}, {y}) is unreachable from start at ({}, {})",
                    map.id, start.0, start.1
                ));
            }
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

    #[cfg(test)]
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

    fn map(rows: &[&str]) -> crate::content::MapDefinition {
        crate::content::MapDefinition {
            id: "runtime_test".to_owned(),
            name: "Runtime Test".to_owned(),
            rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        }
    }

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
        let mut map = map(&["S#X"]);
        map.id = "disconnected".to_owned();

        let error = World::from_map(&map).unwrap_err();
        assert!(error.contains("disconnected"));
        assert!(error.contains("exit at (2, 0) is unreachable"));
    }

    #[test]
    fn runtime_validation_rejects_empty_maps_and_rows() {
        let error = World::from_map(&map(&[])).unwrap_err();
        assert!(error.contains("has no rows"));

        let error = World::from_map(&map(&["SX", ""])).unwrap_err();
        assert!(error.contains("row 1 is empty"));
    }

    #[test]
    fn runtime_validation_rejects_ragged_rows_and_invalid_glyphs() {
        let error = World::from_map(&map(&["SX", "###"])).unwrap_err();
        assert!(error.contains("row 1 has inconsistent width"));

        let error = World::from_map(&map(&["S?X"])).unwrap_err();
        assert!(error.contains("row 0 has invalid tile `?`"));
    }

    #[test]
    fn runtime_validation_rejects_missing_and_multiple_starts() {
        let error = World::from_map(&map(&[".X"])).unwrap_err();
        assert!(error.contains("must contain exactly one start; found 0"));

        let error = World::from_map(&map(&["SSX"])).unwrap_err();
        assert!(error.contains("must contain exactly one start; found 2"));
    }

    #[test]
    fn runtime_validation_rejects_missing_exit() {
        let error = World::from_map(&map(&["S."])).unwrap_err();
        assert!(error.contains("must contain at least one exit"));
    }

    #[test]
    fn runtime_validation_rejects_any_unreachable_exit() {
        let error = World::from_map(&map(&["S.X#X"])).unwrap_err();
        assert!(error.contains("exit at (4, 0) is unreachable"));
    }

    #[test]
    fn runtime_validation_accepts_multiple_reachable_exits_and_doors() {
        let world = World::from_map(&map(&["S.DXX"])).unwrap();
        assert!(world.reachable_exit_from_start());
    }
}
