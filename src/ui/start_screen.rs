use macroquad::prelude::*;

use crate::config::Difficulty;
use crate::mayor::personality::ARCHETYPES;
use super::{GameSetup, StartPhase};

/// Mutable state for the start screen (animation timers, hover, seed input).
pub struct StartScreenState {
    pub setup: GameSetup,
    pub hover_idx: Option<usize>,
    pub seed_input: String,
    pub seed_editing: bool,
    pub anim_timer: f32,
    pub title_pulse: f32,
}

impl StartScreenState {
    pub fn new() -> Self {
        Self {
            setup: GameSetup::default(),
            hover_idx: None,
            seed_input: "42".to_string(),
            seed_editing: false,
            anim_timer: 0.0,
            title_pulse: 0.0,
        }
    }
}

/// Title screen result.
pub enum TitleAction {
    NewGame,
    LoadGame,
}

/// Draw the title screen. Returns Some action when player makes a choice.
pub fn draw_title(state: &mut StartScreenState, dt: f32) -> Option<TitleAction> {
    state.title_pulse += dt * 2.0;

    clear_background(Color::new(0.05, 0.08, 0.04, 1.0));

    let sw = screen_width();
    let sh = screen_height();
    let cx = sw / 2.0;

    // Title
    let title = "SLIDECITY";
    let title_size = 64.0;
    let title_w = measure_text(title, None, title_size as u16, 1.0).width;
    let pulse = (state.title_pulse.sin() * 0.15 + 0.85).max(0.7);
    draw_text(
        title,
        cx - title_w / 2.0,
        sh * 0.3,
        title_size,
        Color::new(0.4 * pulse, 0.8 * pulse, 0.3 * pulse, 1.0),
    );

    // Subtitle
    let sub = "An Autonomous City Simulation";
    let sub_w = measure_text(sub, None, 20, 1.0).width;
    draw_text(
        sub,
        cx - sub_w / 2.0,
        sh * 0.3 + 40.0,
        20.0,
        Color::new(0.5, 0.6, 0.5, 0.8),
    );

    // New Game button
    let btn_w = 220.0;
    let btn_h = 38.0;
    let btn_x = cx - btn_w / 2.0;
    let new_y = sh * 0.55;
    let (mx, my) = mouse_position();

    let new_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= new_y && my <= new_y + btn_h;
    draw_rectangle(btn_x, new_y, btn_w, btn_h,
        if new_hover { Color::new(0.2, 0.4, 0.2, 0.9) } else { Color::new(0.12, 0.25, 0.12, 0.8) });
    draw_rectangle_lines(btn_x, new_y, btn_w, btn_h, 1.0, Color::new(0.3, 0.6, 0.3, 0.7));
    let nl = "New Game  [ENTER]";
    let nw = measure_text(nl, None, 18, 1.0).width;
    draw_text(nl, btn_x + btn_w / 2.0 - nw / 2.0, new_y + 25.0, 18.0,
        Color::new(0.7, 1.0, 0.7, 1.0));

    // Continue button (only if save exists)
    let has_save = crate::game::save_exists();
    if has_save {
        let cont_y = new_y + 50.0;
        let cont_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= cont_y && my <= cont_y + btn_h;
        draw_rectangle(btn_x, cont_y, btn_w, btn_h,
            if cont_hover { Color::new(0.2, 0.35, 0.4, 0.9) } else { Color::new(0.12, 0.2, 0.25, 0.8) });
        draw_rectangle_lines(btn_x, cont_y, btn_w, btn_h, 1.0, Color::new(0.3, 0.5, 0.6, 0.7));
        let cl = "Continue";
        let cw = measure_text(cl, None, 18, 1.0).width;
        draw_text(cl, btn_x + btn_w / 2.0 - cw / 2.0, cont_y + 25.0, 18.0,
            Color::new(0.7, 0.9, 1.0, 1.0));

        if cont_hover && is_mouse_button_pressed(MouseButton::Left) {
            return Some(TitleAction::LoadGame);
        }
    }

    // Version
    draw_text(
        "v1.0.0",
        sw - 70.0,
        sh - 10.0,
        14.0,
        Color::new(0.3, 0.3, 0.3, 1.0),
    );

    if is_key_pressed(KeyCode::Enter)
        || (new_hover && is_mouse_button_pressed(MouseButton::Left))
    {
        return Some(TitleAction::NewGame);
    }
    None
}

/// Draw the mayor selection screen. Returns Some(StartPhase::DifficultySelect) when chosen.
pub fn draw_mayor_select(state: &mut StartScreenState, dt: f32) -> Option<StartPhase> {
    state.anim_timer += dt;

    clear_background(Color::new(0.05, 0.08, 0.04, 1.0));

    let sw = screen_width();
    let cx = sw / 2.0;

    // Header
    let header = "Choose Your Mayor";
    let hw = measure_text(header, None, 36, 1.0).width;
    draw_text(
        header,
        cx - hw / 2.0,
        60.0,
        36.0,
        Color::new(0.7, 0.9, 0.7, 1.0),
    );

    let sub = "Each mayor has unique strengths and weaknesses";
    let sub_w = measure_text(sub, None, 16, 1.0).width;
    draw_text(
        sub,
        cx - sub_w / 2.0,
        85.0,
        16.0,
        Color::new(0.5, 0.6, 0.5, 0.7),
    );

    // 2 rows of 4 mayors
    let card_w = 160.0;
    let card_h = 100.0;
    let gap = 20.0;
    let total_w = 4.0 * card_w + 3.0 * gap;
    let start_x = cx - total_w / 2.0;
    let start_y = 120.0;

    let mx = mouse_position().0;
    let my = mouse_position().1;
    state.hover_idx = None;

    for (i, arch) in ARCHETYPES.iter().enumerate() {
        let col = i % 4;
        let row = i / 4;
        let x = start_x + col as f32 * (card_w + gap);
        let y = start_y + row as f32 * (card_h + gap + 10.0);

        let hovered = mx >= x && mx <= x + card_w && my >= y && my <= y + card_h;
        let selected = state.setup.mayor_idx == i;

        if hovered {
            state.hover_idx = Some(i);
        }

        // Card background
        let bg = if selected {
            Color::new(0.15, 0.35, 0.15, 0.9)
        } else if hovered {
            Color::new(0.12, 0.2, 0.12, 0.8)
        } else {
            Color::new(0.08, 0.12, 0.08, 0.7)
        };
        draw_rectangle(x, y, card_w, card_h, bg);

        // Selection border
        if selected {
            let pulse = (state.anim_timer * 3.0).sin() * 0.2 + 0.8;
            let border = Color::new(0.3, 0.8 * pulse, 0.3, 1.0);
            draw_rectangle_lines(x, y, card_w, card_h, 2.0, border);
        } else if hovered {
            draw_rectangle_lines(x, y, card_w, card_h, 1.0, Color::new(0.3, 0.5, 0.3, 0.6));
        }

        // Emoji (large)
        draw_text(arch.emoji, x + card_w / 2.0 - 16.0, y + 40.0, 36.0, WHITE);

        // Name
        let name_w = measure_text(arch.name, None, 16, 1.0).width;
        draw_text(
            arch.name,
            x + card_w / 2.0 - name_w / 2.0,
            y + 65.0,
            16.0,
            if selected {
                Color::new(0.6, 1.0, 0.6, 1.0)
            } else {
                Color::new(0.7, 0.8, 0.7, 0.9)
            },
        );

        // Number indicator
        let num = format!("{}", i + 1);
        draw_text(&num, x + 6.0, y + 16.0, 14.0, Color::new(0.4, 0.5, 0.4, 0.5));

        // Click to select
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            state.setup.mayor_idx = i;
        }
    }

    // Keyboard selection: 1-8
    for i in 0..8 {
        let key = match i {
            0 => KeyCode::Key1,
            1 => KeyCode::Key2,
            2 => KeyCode::Key3,
            3 => KeyCode::Key4,
            4 => KeyCode::Key5,
            5 => KeyCode::Key6,
            6 => KeyCode::Key7,
            _ => KeyCode::Key8,
        };
        if is_key_pressed(key) {
            state.setup.mayor_idx = i;
        }
    }

    // Seed input
    let seed_y = start_y + 2.0 * (card_h + gap + 10.0) + 20.0;
    draw_text(
        "Map Seed:",
        cx - 120.0,
        seed_y,
        18.0,
        Color::new(0.6, 0.7, 0.6, 0.9),
    );

    // Seed text field
    let field_x = cx - 30.0;
    let field_w = 120.0;
    let field_bg = if state.seed_editing {
        Color::new(0.15, 0.2, 0.15, 0.9)
    } else {
        Color::new(0.1, 0.14, 0.1, 0.7)
    };
    draw_rectangle(field_x, seed_y - 16.0, field_w, 22.0, field_bg);
    draw_rectangle_lines(
        field_x,
        seed_y - 16.0,
        field_w,
        22.0,
        1.0,
        Color::new(0.3, 0.5, 0.3, 0.6),
    );
    draw_text(
        &state.seed_input,
        field_x + 4.0,
        seed_y,
        16.0,
        Color::new(0.8, 0.9, 0.8, 1.0),
    );

    // Click to focus seed input
    if is_mouse_button_pressed(MouseButton::Left) {
        state.seed_editing = mx >= field_x
            && mx <= field_x + field_w
            && my >= seed_y - 16.0
            && my <= seed_y + 6.0;
    }

    // Handle seed text input
    if state.seed_editing {
        if is_key_pressed(KeyCode::Backspace) && !state.seed_input.is_empty() {
            state.seed_input.pop();
        }
        // Number key input for seed
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii_digit() && state.seed_input.len() < 12 {
                state.seed_input.push(ch);
            }
        }
    }

    // Parse seed
    state.setup.seed = state.seed_input.parse::<u64>().unwrap_or(42);

    // Continue button / Enter
    let btn_y = seed_y + 40.0;
    let btn_text = "Continue  [ENTER]";
    let btn_w = 200.0;
    let btn_x = cx - btn_w / 2.0;
    let btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + 36.0;
    let btn_bg = if btn_hover {
        Color::new(0.2, 0.45, 0.2, 0.9)
    } else {
        Color::new(0.12, 0.25, 0.12, 0.8)
    };
    draw_rectangle(btn_x, btn_y, btn_w, 36.0, btn_bg);
    draw_rectangle_lines(
        btn_x,
        btn_y,
        btn_w,
        36.0,
        1.0,
        Color::new(0.3, 0.6, 0.3, 0.8),
    );
    let txt_w = measure_text(btn_text, None, 18, 1.0).width;
    draw_text(
        btn_text,
        btn_x + btn_w / 2.0 - txt_w / 2.0,
        btn_y + 24.0,
        18.0,
        Color::new(0.7, 1.0, 0.7, 1.0),
    );

    if (is_key_pressed(KeyCode::Enter) && !state.seed_editing)
        || (btn_hover && is_mouse_button_pressed(MouseButton::Left))
    {
        return Some(StartPhase::DifficultySelect);
    }

    None
}

/// Draw the difficulty selection screen. Returns true when the player confirms.
pub fn draw_difficulty_select(state: &mut StartScreenState, dt: f32) -> bool {
    state.anim_timer += dt;

    clear_background(Color::new(0.05, 0.08, 0.04, 1.0));

    let sw = screen_width();
    let sh = screen_height();
    let cx = sw / 2.0;
    let mx = mouse_position().0;
    let my = mouse_position().1;

    // Header
    let header = "Select Difficulty";
    let hw = measure_text(header, None, 36, 1.0).width;
    draw_text(
        header,
        cx - hw / 2.0,
        60.0,
        36.0,
        Color::new(0.7, 0.9, 0.7, 1.0),
    );

    // Selected mayor preview
    let arch = &ARCHETYPES[state.setup.mayor_idx];
    let preview = format!("Mayor: {} {}", arch.emoji, arch.name);
    let pw = measure_text(&preview, None, 18, 1.0).width;
    draw_text(
        &preview,
        cx - pw / 2.0,
        95.0,
        18.0,
        Color::new(0.5, 0.7, 0.5, 0.8),
    );

    // Difficulty cards
    let difficulties = [Difficulty::Peaceful, Difficulty::Normal, Difficulty::Harsh];
    let card_w = 260.0;
    let card_h = 120.0;
    let gap = 30.0;
    let total_w = 3.0 * card_w + 2.0 * gap;
    let start_x = cx - total_w / 2.0;
    let start_y = 140.0;

    for (i, &diff) in difficulties.iter().enumerate() {
        let x = start_x + i as f32 * (card_w + gap);
        let y = start_y;
        let selected = state.setup.difficulty == diff;
        let hovered = mx >= x && mx <= x + card_w && my >= y && my <= y + card_h;

        let bg = if selected {
            Color::new(0.15, 0.35, 0.15, 0.9)
        } else if hovered {
            Color::new(0.12, 0.2, 0.12, 0.8)
        } else {
            Color::new(0.08, 0.12, 0.08, 0.7)
        };
        draw_rectangle(x, y, card_w, card_h, bg);

        if selected {
            let pulse = (state.anim_timer * 3.0).sin() * 0.2 + 0.8;
            draw_rectangle_lines(
                x, y, card_w, card_h, 2.0,
                Color::new(0.3, 0.8 * pulse, 0.3, 1.0),
            );
        } else if hovered {
            draw_rectangle_lines(
                x, y, card_w, card_h, 1.0,
                Color::new(0.3, 0.5, 0.3, 0.6),
            );
        }

        // Title
        let name = diff.label();
        let emoji = match diff {
            Difficulty::Peaceful => "🕊️",
            Difficulty::Normal => "⚖️",
            Difficulty::Harsh => "🔥",
        };
        let title = format!("{} {}", emoji, name);
        let tw = measure_text(&title, None, 22, 1.0).width;
        draw_text(
            &title,
            x + card_w / 2.0 - tw / 2.0,
            y + 35.0,
            22.0,
            if selected { WHITE } else { Color::new(0.8, 0.8, 0.8, 0.9) },
        );

        // Description
        let desc = diff.description();
        let dw = measure_text(desc, None, 14, 1.0).width;
        draw_text(
            desc,
            x + card_w / 2.0 - dw / 2.0,
            y + 60.0,
            14.0,
            Color::new(0.6, 0.7, 0.6, 0.8),
        );

        // Key stats
        let (funds, fire, tax) = match diff {
            Difficulty::Peaceful => ("$100k", "14%", "1.5x"),
            Difficulty::Normal => ("$75k", "28%", "1.0x"),
            Difficulty::Harsh => ("$50k", "42%", "0.7x"),
        };
        let stats_line = format!("Funds: {}  Fire: {}  Tax: {}", funds, fire, tax);
        let sl_w = measure_text(&stats_line, None, 12, 1.0).width;
        draw_text(
            &stats_line,
            x + card_w / 2.0 - sl_w / 2.0,
            y + 85.0,
            12.0,
            Color::new(0.5, 0.6, 0.5, 0.6),
        );

        // Number hint
        let num = format!("{}", i + 1);
        draw_text(&num, x + 6.0, y + 16.0, 14.0, Color::new(0.4, 0.5, 0.4, 0.5));

        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            state.setup.difficulty = diff;
        }
    }

    // Keyboard: 1/2/3
    if is_key_pressed(KeyCode::Key1) { state.setup.difficulty = Difficulty::Peaceful; }
    if is_key_pressed(KeyCode::Key2) { state.setup.difficulty = Difficulty::Normal; }
    if is_key_pressed(KeyCode::Key3) { state.setup.difficulty = Difficulty::Harsh; }

    // Speed selector
    let speed_y = start_y + card_h + 50.0;
    draw_text(
        "Starting Speed:",
        cx - 180.0,
        speed_y,
        18.0,
        Color::new(0.6, 0.7, 0.6, 0.9),
    );

    let speed_labels = ["1x", "2x", "4x", "8x"];
    let btn_size = 50.0;
    let speed_start_x = cx - 20.0;
    for (i, label) in speed_labels.iter().enumerate() {
        let x = speed_start_x + i as f32 * (btn_size + 10.0);
        let y = speed_y - 16.0;
        let selected = state.setup.speed_idx == i;
        let hovered = mx >= x && mx <= x + btn_size && my >= y && my <= y + 26.0;

        let bg = if selected {
            Color::new(0.2, 0.4, 0.2, 0.9)
        } else if hovered {
            Color::new(0.15, 0.25, 0.15, 0.8)
        } else {
            Color::new(0.1, 0.15, 0.1, 0.6)
        };
        draw_rectangle(x, y, btn_size, 26.0, bg);
        if selected {
            draw_rectangle_lines(x, y, btn_size, 26.0, 1.0, Color::new(0.3, 0.8, 0.3, 0.8));
        }
        let lw = measure_text(label, None, 16, 1.0).width;
        draw_text(
            label,
            x + btn_size / 2.0 - lw / 2.0,
            y + 18.0,
            16.0,
            if selected { WHITE } else { Color::new(0.6, 0.7, 0.6, 0.8) },
        );

        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            state.setup.speed_idx = i;
        }
    }

    // Start button
    let btn_y = speed_y + 50.0;
    let btn_text = "Start Game  [ENTER]";
    let btn_w = 220.0;
    let btn_x = cx - btn_w / 2.0;
    let btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + 40.0;
    let btn_bg = if btn_hover {
        Color::new(0.2, 0.5, 0.2, 0.95)
    } else {
        Color::new(0.12, 0.3, 0.12, 0.85)
    };
    draw_rectangle(btn_x, btn_y, btn_w, 40.0, btn_bg);
    draw_rectangle_lines(
        btn_x, btn_y, btn_w, 40.0, 2.0,
        Color::new(0.3, 0.7, 0.3, 0.9),
    );
    let txt_w = measure_text(btn_text, None, 20, 1.0).width;
    draw_text(
        btn_text,
        btn_x + btn_w / 2.0 - txt_w / 2.0,
        btn_y + 27.0,
        20.0,
        Color::new(0.7, 1.0, 0.7, 1.0),
    );

    // Back hint
    draw_text(
        "[ESC] Back",
        10.0,
        sh - 10.0,
        14.0,
        Color::new(0.4, 0.4, 0.4, 0.7),
    );

    if is_key_pressed(KeyCode::Escape) {
        // Go back — handled in main
        return false;
    }

    is_key_pressed(KeyCode::Enter)
        || (btn_hover && is_mouse_button_pressed(MouseButton::Left))
}
