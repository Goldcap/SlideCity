use super::{Grid, TileType};

impl Grid {
    /// Count neighbors of a given tile type within manhattan distance `radius`.
    pub fn count_neighbors(&self, col: usize, row: usize, radius: usize, tile: TileType) -> usize {
        let mut count = 0;
        let r = radius as i32;

        for dr in -r..=r {
            for dc in -r..=r {
                if dr == 0 && dc == 0 {
                    continue;
                }
                // Manhattan distance check
                if dr.abs() + dc.abs() > r {
                    continue;
                }
                let nc = col as i32 + dc;
                let nr = row as i32 + dr;
                if nc >= 0 && nc < self.width as i32 && nr >= 0 && nr < self.height as i32
                    && self.get(nc as usize, nr as usize).tile == tile {
                        count += 1;
                    }
            }
        }

        count
    }

    /// Count all non-empty, non-water neighbors within manhattan distance `radius`.
    pub fn count_developed(&self, col: usize, row: usize, radius: usize) -> usize {
        let mut count = 0;
        let r = radius as i32;

        for dr in -r..=r {
            for dc in -r..=r {
                if dr == 0 && dc == 0 {
                    continue;
                }
                if dr.abs() + dc.abs() > r {
                    continue;
                }
                let nc = col as i32 + dc;
                let nr = row as i32 + dr;
                if nc >= 0 && nc < self.width as i32 && nr >= 0 && nr < self.height as i32 {
                    let tile = self.get(nc as usize, nr as usize).tile;
                    if tile != TileType::Empty && tile != TileType::WaterBody {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    /// Check if cell has a road neighbor within distance 1 (4-connected).
    pub fn has_road_neighbor(&self, col: usize, row: usize) -> bool {
        self.count_neighbors(col, row, 1, TileType::Road) > 0
    }

    /// Count road neighbors in the 4 cardinal directions (for road alignment checks).
    pub fn road_neighbors_cardinal(&self, col: usize, row: usize) -> [(bool, bool); 2] {
        let has_n = row > 0 && self.get(col, row - 1).tile == TileType::Road;
        let has_s = row + 1 < self.height && self.get(col, row + 1).tile == TileType::Road;
        let has_w = col > 0 && self.get(col - 1, row).tile == TileType::Road;
        let has_e = col + 1 < self.width && self.get(col + 1, row).tile == TileType::Road;
        [(has_n, has_s), (has_w, has_e)]
    }

    /// Check if roads are aligned (forming a line, not a corner).
    pub fn roads_aligned(&self, col: usize, row: usize) -> bool {
        let [(n, s), (w, e)] = self.road_neighbors_cardinal(col, row);
        (n && s && !w && !e) || (w && e && !n && !s)
    }
}
