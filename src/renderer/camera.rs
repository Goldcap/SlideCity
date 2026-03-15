use macroquad::prelude::*;

pub const ZOOM_LEVELS: &[f32] = &[0.5, 0.75, 1.0, 1.5, 2.0];

pub struct GameCamera {
    pub target: Vec2,
    pub zoom: f32,
    target_target: Vec2,
    target_zoom: f32,

    // Screen shake
    shake_intensity: f32,
    shake_timer: f32,

    // Smooth pan request
    pan_target: Option<Vec2>,
    pan_speed: f32,

    // Drag state
    last_mouse: Option<Vec2>,
}

impl GameCamera {
    pub fn new(initial_target: Vec2) -> Self {
        Self {
            target: initial_target,
            zoom: 0.75,
            target_target: initial_target,
            target_zoom: 0.75,
            shake_intensity: 0.0,
            shake_timer: 0.0,
            pan_target: None,
            pan_speed: 6.0,
            last_mouse: None,
        }
    }

    pub fn update(&mut self, dt: f32) {
        // Smooth lerp toward target
        let lerp_speed = self.pan_speed * dt;
        self.target = self.target.lerp(self.target_target, lerp_speed.min(1.0));
        self.zoom += (self.target_zoom - self.zoom) * (6.0 * dt).min(1.0);

        // Auto-pan to requested location
        if let Some(pan) = self.pan_target {
            self.target_target = self.target_target.lerp(pan, (4.0 * dt).min(1.0));
            if self.target_target.distance(pan) < 1.0 {
                self.pan_target = None;
            }
        }

        // Screen shake decay
        if self.shake_timer > 0.0 {
            self.shake_timer -= dt;
            if self.shake_timer <= 0.0 {
                self.shake_intensity = 0.0;
                self.shake_timer = 0.0;
            }
        }
    }

    pub fn handle_input(&mut self, dt: f32) {
        // Zoom: scroll wheel
        let (_, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            let dir = wheel_y.signum();
            let cur_idx = ZOOM_LEVELS
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    ((**a) - self.target_zoom)
                        .abs()
                        .partial_cmp(&((**b) - self.target_zoom).abs())
                        .unwrap()
                })
                .map(|(i, _)| i)
                .unwrap_or(2);
            let new_idx = if dir > 0.0 {
                (cur_idx + 1).min(ZOOM_LEVELS.len() - 1)
            } else {
                cur_idx.saturating_sub(1)
            };
            self.target_zoom = ZOOM_LEVELS[new_idx];
        }

        // Pan: click-drag
        let mouse = vec2(mouse_position().0, mouse_position().1);
        if is_mouse_button_down(MouseButton::Left) {
            if let Some(prev) = self.last_mouse {
                let delta = mouse - prev;
                self.target_target.x -= delta.x / self.zoom;
                self.target_target.y -= delta.y / self.zoom;
                self.pan_target = None; // Cancel auto-pan on manual drag
            }
            self.last_mouse = Some(mouse);
        } else {
            self.last_mouse = None;
        }

        // Pan: arrow keys / WASD
        let pan_speed = 400.0 / self.zoom * dt;
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.target_target.x -= pan_speed;
            self.pan_target = None;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.target_target.x += pan_speed;
            self.pan_target = None;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            self.target_target.y -= pan_speed;
            self.pan_target = None;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            self.target_target.y += pan_speed;
            self.pan_target = None;
        }
    }

    /// Request smooth pan to a world position (e.g., mayor action location).
    pub fn pan_to(&mut self, world_pos: Vec2) {
        self.pan_target = Some(world_pos);
        self.pan_speed = 6.0;
    }

    /// Snap-pan with screen shake (e.g., disaster).
    pub fn shake_at(&mut self, world_pos: Vec2, intensity: f32, duration: f32) {
        self.target_target = world_pos;
        self.target = world_pos; // Snap, don't lerp
        self.shake_intensity = intensity;
        self.shake_timer = duration;
        self.pan_target = None;
    }

    /// Get the Camera2D for macroquad rendering, including shake offset.
    pub fn to_macroquad_camera(&self) -> Camera2D {
        let mut offset = self.target;

        // Apply screen shake
        if self.shake_timer > 0.0 {
            let t = self.shake_timer;
            offset.x += (t * 37.0).sin() * self.shake_intensity;
            offset.y += (t * 53.0).cos() * self.shake_intensity * 0.7;
        }

        Camera2D {
            target: offset,
            zoom: vec2(
                self.zoom / screen_width() * 2.0,
                self.zoom / screen_height() * 2.0,
            ),
            ..Default::default()
        }
    }
}
