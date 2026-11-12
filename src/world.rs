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
        let rows = [
            "###############",
            "#S...D...D...X#",
            "###.#########.#",
            "#...#.........#",
            "#...#########.#",
            "#############.#",
            "###############",
        ];
        let width = rows[0].len();
        let mut start = (1, 1);
        let mut tiles = Vec::with_capacity(width * rows.len());
        for (y, row) in rows.iter().enumerate() {
            for (x, cell) in row.bytes().enumerate() {
                tiles.push(match cell {
                    b'#' => Tile::Wall,
                    b'D' => Tile::Door { open: false },
                    b'X' => Tile::Exit,
                    b'S' => {
                        start = (x, y);
                        Tile::Floor
                    }
                    _ => Tile::Floor,
                });
            }
        }
        Self {
            width,
            height: rows.len(),
            tiles,
            start,
        }
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
        let mut visited = vec![false; self.tiles.len()];
        let mut queue = VecDeque::from([self.start]);
        visited[self.start.1 * self.width + self.start.0] = true;
        while let Some((x, y)) = queue.pop_front() {
            if self.tile_at(x as isize, y as isize) == Tile::Exit {
                return true;
            }
            for (dx, dy) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                let tile = self.tile_at(nx, ny);
                if matches!(tile, Tile::Wall) {
                    continue;
                }
                let index = ny as usize * self.width + nx as usize;
                if !visited[index] {
                    visited[index] = true;
                    queue.push_back((nx as usize, ny as usize));
                }
            }
        }
        false
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
}
#[cfg(test)]
use std::collections::VecDeque;
