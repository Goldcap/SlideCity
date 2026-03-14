use macroquad::prelude::*;
use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::grid::{Grid, TileType};
use super::iso::grid_to_screen;

struct Particle {
    pos: Vec2,
    vel: Vec2,
    lifetime: f32,
    max_lifetime: f32,
    color: Color,
    size: f32,
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    spawn_timer: f32,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(512),
            spawn_timer: 0.0,
        }
    }

    /// Spawn particles based on grid state (called periodically, not every frame).
    pub fn spawn_from_grid(&mut self, grid: &Grid, rng: &mut SmallRng) {
        self.spawn_timer += 1.0;
        if self.spawn_timer < 3.0 {
            return; // Spawn every ~3 calls
        }
        self.spawn_timer = 0.0;

        for row in 0..grid.height {
            for col in 0..grid.width {
                let cell = grid.get(col, row);
                let height = cell.tile.height_floors(cell.age);
                let base = grid_to_screen(col, row, height);

                match cell.tile {
                    // Smoke from mature industrial
                    TileType::Industrial if cell.age > 30 => {
                        if rng.gen::<f32>() < 0.15 {
                            self.particles.push(Particle {
                                pos: vec2(
                                    base.x + rng.gen_range(-5.0..5.0),
                                    base.y - height * 8.0 - 10.0,
                                ),
                                vel: vec2(rng.gen_range(-3.0..3.0), rng.gen_range(-15.0..-8.0)),
                                lifetime: 0.0,
                                max_lifetime: rng.gen_range(1.0..2.5),
                                color: Color::new(0.5, 0.5, 0.5, 0.6),
                                size: rng.gen_range(3.0..6.0),
                            });
                        }
                    }
                    // Sparks from fire
                    TileType::Fire => {
                        if rng.gen::<f32>() < 0.4 {
                            self.particles.push(Particle {
                                pos: vec2(
                                    base.x + rng.gen_range(-8.0..8.0),
                                    base.y - 5.0,
                                ),
                                vel: vec2(
                                    rng.gen_range(-10.0..10.0),
                                    rng.gen_range(-30.0..-10.0),
                                ),
                                lifetime: 0.0,
                                max_lifetime: rng.gen_range(0.3..0.8),
                                color: Color::new(1.0, rng.gen_range(0.3..0.8), 0.0, 1.0),
                                size: rng.gen_range(1.5..3.0),
                            });
                        }
                    }
                    // Dust from new construction
                    TileType::Residential | TileType::Commercial | TileType::Industrial
                        if cell.age < 5 =>
                    {
                        if rng.gen::<f32>() < 0.2 {
                            self.particles.push(Particle {
                                pos: vec2(
                                    base.x + rng.gen_range(-10.0..10.0),
                                    base.y + rng.gen_range(-5.0..5.0),
                                ),
                                vel: vec2(
                                    rng.gen_range(-8.0..8.0),
                                    rng.gen_range(-5.0..2.0),
                                ),
                                lifetime: 0.0,
                                max_lifetime: rng.gen_range(0.5..1.2),
                                color: Color::new(0.6, 0.55, 0.4, 0.5),
                                size: rng.gen_range(2.0..4.0),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        // Cap particle count
        if self.particles.len() > 500 {
            self.particles.drain(0..(self.particles.len() - 500));
        }
    }

    /// Update all particles (call every frame).
    pub fn update(&mut self, dt: f32) {
        for p in self.particles.iter_mut() {
            p.pos += p.vel * dt;
            p.lifetime += dt;
            // Smoke rises and slows
            p.vel.y *= 0.98;
            p.vel.x *= 0.96;
        }

        // Remove dead particles
        self.particles.retain(|p| p.lifetime < p.max_lifetime);
    }

    /// Draw all particles (call in world space, before set_default_camera).
    pub fn draw(&self) {
        for p in &self.particles {
            let alpha = 1.0 - (p.lifetime / p.max_lifetime);
            let size = p.size * (1.0 + p.lifetime * 0.5); // Grow slightly over time
            let color = Color::new(p.color.r, p.color.g, p.color.b, p.color.a * alpha);
            draw_circle(p.pos.x, p.pos.y, size, color);
        }
    }
}
