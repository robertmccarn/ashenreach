use std::collections::VecDeque;

/// Returns the cells reachable from `start` in a rectangular grid.
///
/// Callers provide the traversal rule so content validation and runtime tiles
/// can share this flood fill without sharing initialization or storage types.
pub(crate) fn flood_fill(
    width: usize,
    height: usize,
    start: (usize, usize),
    mut is_traversable: impl FnMut(usize, usize) -> bool,
) -> Option<Vec<bool>> {
    let cell_count = width.checked_mul(height)?;
    let mut visited = vec![false; cell_count];
    if width == 0 || height == 0 || start.0 >= width || start.1 >= height {
        return Some(visited);
    }

    let start_index = start.1 * width + start.0;
    if !is_traversable(start.0, start.1) {
        return Some(visited);
    }
    visited[start_index] = true;
    let mut queue = VecDeque::from([start]);

    while let Some((x, y)) = queue.pop_front() {
        let neighbors = [
            x.checked_add(1).filter(|&nx| nx < width).map(|nx| (nx, y)),
            x.checked_sub(1).map(|nx| (nx, y)),
            y.checked_add(1).filter(|&ny| ny < height).map(|ny| (x, ny)),
            y.checked_sub(1).map(|ny| (x, ny)),
        ];
        for (nx, ny) in neighbors.into_iter().flatten() {
            let index = ny * width + nx;
            if !visited[index] && is_traversable(nx, ny) {
                visited[index] = true;
                queue.push_back((nx, ny));
            }
        }
    }

    Some(visited)
}

#[cfg(test)]
mod tests {
    use super::flood_fill;

    #[test]
    fn flood_fill_handles_empty_and_out_of_bounds_grids() {
        assert_eq!(flood_fill(0, 0, (0, 0), |_, _| true), Some(vec![]));
        assert_eq!(flood_fill(2, 1, (2, 0), |_, _| true), Some(vec![false; 2]));
    }
}
