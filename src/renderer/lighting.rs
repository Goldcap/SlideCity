use macroquad::prelude::*;

/// Day/night cycle: purely cosmetic color tint.
/// Full cycle = 120 seconds (2 minutes real-time).
pub struct DayNightCycle {
    time: f32, // 0.0 - 1.0 representing full day cycle
}

impl DayNightCycle {
    pub fn new() -> Self {
        Self { time: 0.25 } // Start at "morning"
    }

    pub fn update(&mut self, dt: f32) {
        const CYCLE_DURATION: f32 = 120.0;
        self.time = (self.time + dt / CYCLE_DURATION) % 1.0;
    }

    /// Get the current tint color to multiply with all world rendering.
    /// Dawn (warm gold) → Day (white) → Dusk (warm orange) → Night (cool blue)
    pub fn tint(&self) -> Color {
        let t = self.time;

        let (r, g, b) = if t < 0.2 {
            // Night → Dawn (0.0 - 0.2)
            let s = t / 0.2;
            lerp_rgb((0.5, 0.5, 0.7), (1.0, 0.85, 0.7), s)
        } else if t < 0.3 {
            // Dawn → Day (0.2 - 0.3)
            let s = (t - 0.2) / 0.1;
            lerp_rgb((1.0, 0.85, 0.7), (1.0, 1.0, 0.98), s)
        } else if t < 0.7 {
            // Day (0.3 - 0.7)
            (1.0, 1.0, 0.98)
        } else if t < 0.8 {
            // Day → Dusk (0.7 - 0.8)
            let s = (t - 0.7) / 0.1;
            lerp_rgb((1.0, 1.0, 0.98), (1.0, 0.75, 0.55), s)
        } else if t < 0.9 {
            // Dusk → Night (0.8 - 0.9)
            let s = (t - 0.8) / 0.1;
            lerp_rgb((1.0, 0.75, 0.55), (0.5, 0.5, 0.7), s)
        } else {
            // Night (0.9 - 1.0)
            (0.5, 0.5, 0.7)
        };

        Color::new(r, g, b, 1.0)
    }

    /// Get a label for the current time of day.
    pub fn phase_label(&self) -> &'static str {
        let t = self.time;
        if t < 0.2 { "Night" }
        else if t < 0.3 { "Dawn" }
        else if t < 0.7 { "Day" }
        else if t < 0.8 { "Dusk" }
        else { "Night" }
    }
}

fn lerp_rgb(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}
