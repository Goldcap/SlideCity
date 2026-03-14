use macroquad::prelude::*;

use super::InfluenceState;

/// Draw the influence/IP display and disaster button in the right panel.
/// `panel_x` is the left edge of the right panel.
/// Returns true if the disaster button was clicked and is available.
pub fn draw_influence(state: &InfluenceState, panel_x: f32) -> bool {
    let sh = screen_height();
    let panel_w = 300.0;

    // Position at bottom of right panel
    let section_y = sh - 160.0;

    // Separator
    draw_line(
        panel_x + 10.0, section_y,
        panel_x + panel_w - 10.0, section_y,
        1.0,
        Color::new(0.2, 0.35, 0.2, 0.5),
    );

    // IP Display
    let ip_y = section_y + 25.0;
    draw_text(
        "Influence Points",
        panel_x + 10.0,
        ip_y,
        14.0,
        Color::new(0.5, 0.7, 0.5, 0.7),
    );

    // IP count (large)
    let ip_text = format!("{} IP", state.ip);
    draw_text(
        &ip_text,
        panel_x + 10.0,
        ip_y + 22.0,
        22.0,
        Color::new(0.7, 0.95, 0.5, 1.0),
    );

    // Tier costs
    let tier_y = ip_y + 42.0;
    let tiers = [
        ("Suggest", 1u32, Color::new(0.5, 0.7, 0.5, 0.7)),
        ("Council", 3, Color::new(0.5, 0.6, 0.8, 0.7)),
        ("Audience", 5, Color::new(0.8, 0.6, 0.5, 0.7)),
    ];
    let mut tx = panel_x + 10.0;
    for (name, cost, color) in &tiers {
        let available = state.ip >= *cost;
        let alpha = if available { 1.0 } else { 0.4 };
        let label = format!("{}: {}IP", name, cost);
        draw_text(
            &label,
            tx,
            tier_y,
            12.0,
            Color::new(color.r, color.g, color.b, alpha),
        );
        tx += measure_text(&label, None, 12, 1.0).width + 12.0;
    }

    // Disaster Button
    let btn_y = tier_y + 20.0;
    let btn_w = panel_w - 20.0;
    let btn_h = 32.0;
    let btn_x = panel_x + 10.0;

    let on_cooldown = state.disaster_cooldown > 0.0;
    let maxed_out = state.disasters_this_year >= 2;
    let available = !on_cooldown && !maxed_out;

    let (mx, my) = mouse_position();
    let hovered = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;

    let btn_color = if !available {
        Color::new(0.2, 0.15, 0.15, 0.5)
    } else if hovered {
        Color::new(0.6, 0.2, 0.1, 0.9)
    } else {
        Color::new(0.4, 0.15, 0.08, 0.8)
    };

    draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
    draw_rectangle_lines(
        btn_x, btn_y, btn_w, btn_h,
        1.0,
        if available {
            Color::new(0.8, 0.3, 0.1, 0.8)
        } else {
            Color::new(0.3, 0.2, 0.15, 0.4)
        },
    );

    let btn_label = if on_cooldown {
        format!("FIRE  (cooldown {:.0}s)", state.disaster_cooldown)
    } else if maxed_out {
        "FIRE  (max 2/year)".to_string()
    } else {
        "FIRE  [+2 IP]".to_string()
    };

    let lw = measure_text(&btn_label, None, 16, 1.0).width;
    draw_text(
        &btn_label,
        btn_x + btn_w / 2.0 - lw / 2.0,
        btn_y + 22.0,
        16.0,
        if available {
            Color::new(1.0, 0.6, 0.3, 1.0)
        } else {
            Color::new(0.5, 0.4, 0.3, 0.5)
        },
    );

    // Speed slider label at bottom
    let speed_hint_y = btn_y + btn_h + 18.0;
    draw_text(
        "1-4: speed | Scroll: zoom | Drag: pan",
        panel_x + 10.0,
        speed_hint_y,
        12.0,
        Color::new(0.35, 0.4, 0.35, 0.5),
    );

    // FPS
    draw_text(
        &format!("FPS: {}", get_fps()),
        panel_x + panel_w - 65.0,
        speed_hint_y,
        12.0,
        Color::new(0.35, 0.4, 0.35, 0.5),
    );

    available && hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Draw the in-game speed slider at the bottom of the screen.
/// Returns Some(new_speed_idx) if the player clicked a speed button.
pub fn draw_speed_slider(current_idx: usize) -> Option<usize> {
    let sw = screen_width();
    let sh = screen_height();

    // Position: bottom center, above the minimap area
    let labels = ["1x", "2x", "4x", "8x"];
    let btn_w = 36.0;
    let btn_h = 22.0;
    let gap = 4.0;
    let total_w = labels.len() as f32 * (btn_w + gap) - gap;
    let start_x = sw / 2.0 - total_w / 2.0;
    let y = sh - 30.0;

    draw_text(
        "Speed:",
        start_x - 55.0,
        y + 16.0,
        14.0,
        Color::new(0.4, 0.5, 0.4, 0.6),
    );

    let (mx, my) = mouse_position();
    let mut result = None;

    for (i, label) in labels.iter().enumerate() {
        let x = start_x + i as f32 * (btn_w + gap);
        let selected = current_idx == i;
        let hovered = mx >= x && mx <= x + btn_w && my >= y && my <= y + btn_h;

        let bg = if selected {
            Color::new(0.15, 0.35, 0.15, 0.85)
        } else if hovered {
            Color::new(0.12, 0.2, 0.12, 0.7)
        } else {
            Color::new(0.08, 0.12, 0.08, 0.5)
        };

        draw_rectangle(x, y, btn_w, btn_h, bg);
        if selected {
            draw_rectangle_lines(x, y, btn_w, btn_h, 1.0, Color::new(0.3, 0.7, 0.3, 0.8));
        }

        let lw = measure_text(label, None, 13, 1.0).width;
        draw_text(
            label,
            x + btn_w / 2.0 - lw / 2.0,
            y + 16.0,
            13.0,
            if selected {
                Color::new(0.7, 1.0, 0.7, 1.0)
            } else {
                Color::new(0.5, 0.6, 0.5, 0.7)
            },
        );

        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            result = Some(i);
        }
    }

    result
}
