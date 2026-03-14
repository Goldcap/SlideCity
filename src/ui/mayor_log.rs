use macroquad::prelude::*;

use crate::mayor::Mayor;

/// Draw the right-side mayor panel with identity and scrolling log.
/// Returns the panel X coordinate for other UI elements to use.
pub fn draw_mayor_panel(mayor: &Mayor) -> f32 {
    let sw = screen_width();
    let sh = screen_height();
    let panel_w = 300.0;
    let panel_x = sw - panel_w;

    // Panel background
    draw_rectangle(panel_x, 0.0, panel_w, sh, Color::new(0.02, 0.04, 0.02, 0.8));
    draw_line(panel_x, 0.0, panel_x, sh, 1.0, Color::new(0.2, 0.3, 0.2, 0.5));

    let p = mayor.personality();

    // Mayor identity
    draw_text(
        &format!("{} {}", p.emoji, p.name),
        panel_x + 10.0,
        28.0,
        22.0,
        WHITE,
    );

    // Phase + mayor number
    draw_text(
        &format!("{:?} | Mayor #{}", mayor.phase, mayor.mayor_number),
        panel_x + 10.0,
        48.0,
        14.0,
        Color::new(0.5, 0.65, 0.5, 0.8),
    );

    // Separator
    draw_line(
        panel_x + 10.0, 58.0,
        panel_x + panel_w - 10.0, 58.0,
        1.0,
        Color::new(0.2, 0.35, 0.2, 0.5),
    );

    // Log label
    draw_text(
        "Mayor's Log",
        panel_x + 10.0,
        76.0,
        13.0,
        Color::new(0.4, 0.55, 0.4, 0.6),
    );

    // Log entries (last 7, newest first)
    let entries = mayor.log.last_n(7);
    for (i, entry) in entries.iter().enumerate() {
        let y = 96.0 + i as f32 * 62.0;

        // Fade older entries: newest = 1.0, oldest = ~0.4
        let opacity = if i == 0 {
            1.0
        } else {
            (1.0 - i as f32 * 0.1).max(0.4)
        };

        let header_color = Color::new(0.5, 0.75, 0.5, opacity);
        let text_color = Color::new(0.85, 0.9, 0.85, opacity);

        // Header: emoji + year + season
        draw_text(
            &format!("{} Year {}, {}", entry.emoji, entry.year, entry.season),
            panel_x + 10.0,
            y,
            13.0,
            header_color,
        );

        // Word-wrap the text (~36 chars per line for 300px panel)
        let max_chars = 36;
        let text = &entry.text;
        let lines = wrap_text(text, max_chars);
        for (li, line) in lines.iter().enumerate().take(3) {
            draw_text(
                line,
                panel_x + 10.0,
                y + 16.0 + li as f32 * 14.0,
                14.0,
                text_color,
            );
        }
    }

    panel_x
}

/// Simple word-wrap: break at spaces, max chars per line.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() > max_chars {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
