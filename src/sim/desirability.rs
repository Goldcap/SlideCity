use crate::grid::{Grid, TileType};

/// Per-cell desirability score grid. Drives WHERE buildings grow.
/// Range: roughly -100.0 to +100.0 (additive scoring).
pub struct DesirabilityGrid {
    pub values: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl DesirabilityGrid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            values: vec![0.0; width * height],
            width,
            height,
        }
    }

    #[inline]
    pub fn get(&self, col: usize, row: usize) -> f32 {
        self.values[row * self.width + col]
    }

    /// Full recompute of desirability for all cells.
    pub fn recompute(&mut self, grid: &Grid) {
        let w = self.width;
        let h = self.height;

        // Reset all to base value
        for v in self.values.iter_mut() {
            *v = 0.0;
        }

        // Apply all factors from source cells
        for row in 0..h {
            for col in 0..w {
                let cell = grid.get(col, row);
                match cell.tile {
                    // Parks boost desirability in radius 5
                    TileType::Park => {
                        self.apply_radius(col, row, 5, &[15.0, 10.0, 7.0, 4.0, 2.0]);
                    }
                    // Industrial creates pollution (negative for residential)
                    TileType::Industrial => {
                        self.apply_radius(col, row, 3, &[-20.0, -12.0, -5.0]);
                    }
                    // Abandoned buildings create blight
                    TileType::Abandoned => {
                        self.apply_radius(col, row, 2, &[-10.0, -5.0]);
                    }
                    // Water proximity is pleasant
                    TileType::WaterBody => {
                        self.apply_radius(col, row, 3, &[8.0, 5.0, 2.0]);
                    }
                    _ => {}
                }
            }
        }

        // Per-cell bonuses
        for row in 0..h {
            for col in 0..w {
                let idx = row * w + col;
                let cell = grid.get(col, row);

                // Road access bonus (essential for growth)
                if grid.has_road_neighbor(col, row) {
                    self.values[idx] += 20.0;
                }

                // Utility bonuses
                if cell.has_power {
                    self.values[idx] += 10.0;
                }
                if cell.has_water {
                    self.values[idx] += 5.0;
                }

                // Elevation bonus (higher = more desirable for residential)
                if cell.terrain_height > 0.6 {
                    self.values[idx] += 5.0;
                }
            }
        }
    }

    /// Apply a value at decreasing amounts within a radius (Manhattan distance).
    fn apply_radius(&mut self, center_col: usize, center_row: usize, radius: usize, values: &[f32]) {
        let w = self.width;
        let h = self.height;

        let min_row = center_row.saturating_sub(radius);
        let max_row = (center_row + radius).min(h - 1);
        let min_col = center_col.saturating_sub(radius);
        let max_col = (center_col + radius).min(w - 1);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let dist = center_col.abs_diff(col) + center_row.abs_diff(row);
                if dist == 0 || dist > radius {
                    continue;
                }
                if let Some(&val) = values.get(dist - 1) {
                    self.values[row * w + col] += val;
                }
            }
        }
    }
}
