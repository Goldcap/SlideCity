use macroquad::prelude::*;

use crate::influence::{ActionCategory, InfluenceState, MayorResponse};
use super::InfluenceModal;

/// User action returned from modal interaction.
#[derive(Clone, Debug, PartialEq)]
pub enum InfluenceAction {
    None,
    /// Fire disaster button clicked.
    DisasterButton,
    /// Open a tier modal.
    OpenSuggestion,
    OpenCouncil,
    OpenAudience,
    OpenBuyIP,
    /// Player picked an action in suggestion box.
    SuggestionPick(ActionCategory),
    /// Player picked an option in council vote.
    CouncilPick(usize),
    /// Player submitted audience text.
    AudienceSubmit(String),
    /// Player confirmed IP purchase.
    BuyIPConfirm,
    /// Close any modal.
    CloseModal,
}

/// Draw the influence panel in the right sidebar.
/// Returns an action if the player interacted with something.
pub fn draw_influence_panel(
    state: &InfluenceState,
    _modal: &InfluenceModal,
    panel_x: f32,
) -> InfluenceAction {
    let sh = screen_height();
    let panel_w = 300.0;
    let (mx, my) = mouse_position();

    // Position at bottom of right panel
    let section_y = sh - 200.0;

    // Separator
    draw_line(
        panel_x + 10.0, section_y,
        panel_x + panel_w - 10.0, section_y,
        1.0,
        Color::new(0.2, 0.35, 0.2, 0.5),
    );

    // IP Display
    let ip_y = section_y + 20.0;
    draw_text(
        "Influence Points",
        panel_x + 10.0,
        ip_y,
        14.0,
        Color::new(0.5, 0.7, 0.5, 0.7),
    );

    let ip_text = format!("{} IP", state.ip);
    draw_text(
        &ip_text,
        panel_x + 10.0,
        ip_y + 20.0,
        22.0,
        Color::new(0.7, 0.95, 0.5, 1.0),
    );

    // Compliance boost indicator
    if state.compliance_boost > 0 {
        draw_text(
            &format!("Boost: {} decisions", state.compliance_boost),
            panel_x + 120.0,
            ip_y + 20.0,
            13.0,
            Color::new(0.9, 0.8, 0.3, 0.8),
        );
    }

    // Buy IP button
    let buy_y = ip_y + 32.0;
    let buy_btn = draw_small_btn(
        panel_x + 10.0, buy_y, 80.0, 20.0,
        "Buy 1IP ($5k)",
        Color::new(0.4, 0.5, 0.3, 0.8),
        mx, my,
    );

    // Tier buttons
    let tier_y = buy_y + 28.0;
    let btn_w = 88.0;
    let gap = 4.0;

    let suggest_btn = draw_tier_btn(
        panel_x + 10.0, tier_y, btn_w, 26.0,
        "Suggest [1]", 1, state,
        Color::new(0.3, 0.55, 0.3, 0.85),
        mx, my,
    );

    let council_btn = draw_tier_btn(
        panel_x + 10.0 + btn_w + gap, tier_y, btn_w, 26.0,
        "Council [3]", 3, state,
        Color::new(0.3, 0.4, 0.65, 0.85),
        mx, my,
    );

    let audience_btn = draw_tier_btn(
        panel_x + 10.0 + 2.0 * (btn_w + gap), tier_y, btn_w, 26.0,
        "Audience [5]", 5, state,
        Color::new(0.6, 0.4, 0.3, 0.85),
        mx, my,
    );

    // Disaster button
    let fire_y = tier_y + 34.0;
    let fire_clicked = draw_disaster_btn(state, panel_x + 10.0, fire_y, panel_w - 20.0, 28.0, mx, my);

    // Controls hint + FPS
    let hint_y = fire_y + 36.0;
    draw_text(
        "1-4: speed | Q/E: rotate | Scroll: zoom",
        panel_x + 10.0,
        hint_y,
        12.0,
        Color::new(0.35, 0.4, 0.35, 0.5),
    );
    draw_text(
        &format!("FPS: {}", get_fps()),
        panel_x + panel_w - 65.0,
        hint_y,
        12.0,
        Color::new(0.35, 0.4, 0.35, 0.5),
    );

    // Determine action from buttons
    if fire_clicked {
        return InfluenceAction::DisasterButton;
    }
    if buy_btn {
        return InfluenceAction::OpenBuyIP;
    }
    if suggest_btn {
        return InfluenceAction::OpenSuggestion;
    }
    if council_btn {
        return InfluenceAction::OpenCouncil;
    }
    if audience_btn {
        return InfluenceAction::OpenAudience;
    }

    InfluenceAction::None
}

/// Draw the active modal overlay. Returns an action if player interacts.
pub fn draw_modal(modal: &mut InfluenceModal) -> InfluenceAction {
    match modal {
        InfluenceModal::None => InfluenceAction::None,
        InfluenceModal::SuggestionBox => draw_suggestion_modal(),
        InfluenceModal::CouncilVote { candidates } => draw_council_modal(candidates),
        InfluenceModal::Audience { input, response, waiting } => {
            draw_audience_modal(input, response, *waiting)
        }
        InfluenceModal::BuyIP => draw_buy_ip_modal(),
    }
}

/// Draw the response overlay after an influence action resolves.
pub fn draw_response_toast(response: &MayorResponse, timer: f32) {
    if timer <= 0.0 {
        return;
    }

    let sw = screen_width();
    let sh = screen_height();
    let alpha = (timer / 3.0).min(1.0);

    let w = 400.0;
    let h = 80.0;
    let x = sw / 2.0 - w / 2.0;
    let y = sh * 0.15;

    // Background
    draw_rectangle(x, y, w, h, Color::new(0.05, 0.08, 0.05, 0.9 * alpha));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(0.3, 0.5, 0.3, 0.7 * alpha));

    // Tag
    let (tag, tag_color) = match response {
        MayorResponse::Comply(..) => ("[COMPLY]", Color::new(0.3, 0.9, 0.3, alpha)),
        MayorResponse::Ignore(..) => ("[IGNORE]", Color::new(0.7, 0.7, 0.3, alpha)),
        MayorResponse::Argue(..) => ("[ARGUE]", Color::new(0.9, 0.4, 0.3, alpha)),
        MayorResponse::Override(..) => ("[OVERRIDE]", Color::new(0.9, 0.5, 0.2, alpha)),
    };
    draw_text(tag, x + 10.0, y + 22.0, 16.0, tag_color);

    // Response text (word-wrap at ~50 chars)
    let text = response.text();
    let max_chars = 52;
    let mut line_y = y + 42.0;
    let mut start = 0;
    while start < text.len() {
        let end = (start + max_chars).min(text.len());
        let slice = if end < text.len() {
            // Try to break at space
            let sub = &text[start..end];
            if let Some(space) = sub.rfind(' ') {
                &text[start..start + space]
            } else {
                sub
            }
        } else {
            &text[start..end]
        };
        draw_text(slice, x + 10.0, line_y, 15.0, Color::new(0.85, 0.9, 0.85, alpha));
        line_y += 16.0;
        start += slice.len();
        // Skip the space we broke at
        if start < text.len() && text.as_bytes().get(start) == Some(&b' ') {
            start += 1;
        }
    }
}

// ===== MODAL IMPLEMENTATIONS =====

fn draw_suggestion_modal() -> InfluenceAction {
    let (action, _) = draw_modal_frame("Suggestion Box", "Pick an action to suggest to the mayor:");

    if action == InfluenceAction::CloseModal {
        return action;
    }

    let sw = screen_width();
    let sh = screen_height();
    let modal_w = 440.0;
    let modal_x = sw / 2.0 - modal_w / 2.0;
    let start_y = sh / 2.0 - 100.0;
    let (mx, my) = mouse_position();

    for (i, &cat) in ActionCategory::ALL.iter().enumerate() {
        let y = start_y + i as f32 * 32.0;
        let label = format!("{} {}", cat.emoji(), cat.label());
        let hovered = mx >= modal_x + 20.0
            && mx <= modal_x + modal_w - 20.0
            && my >= y
            && my <= y + 28.0;

        let bg = if hovered {
            Color::new(0.15, 0.3, 0.15, 0.9)
        } else {
            Color::new(0.08, 0.12, 0.08, 0.6)
        };
        draw_rectangle(modal_x + 20.0, y, modal_w - 40.0, 28.0, bg);
        if hovered {
            draw_rectangle_lines(
                modal_x + 20.0, y, modal_w - 40.0, 28.0,
                1.0, Color::new(0.3, 0.6, 0.3, 0.8),
            );
        }
        draw_text(
            &label,
            modal_x + 30.0,
            y + 20.0,
            16.0,
            if hovered { WHITE } else { Color::new(0.7, 0.8, 0.7, 0.9) },
        );

        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            return InfluenceAction::SuggestionPick(cat);
        }
    }

    InfluenceAction::None
}

fn draw_council_modal(candidates: &[ActionCategory; 3]) -> InfluenceAction {
    let (action, _) = draw_modal_frame("Council Vote", "The mayor presents 3 options. Choose wisely:");

    if action == InfluenceAction::CloseModal {
        return action;
    }

    let sw = screen_width();
    let sh = screen_height();
    let modal_w = 440.0;
    let modal_x = sw / 2.0 - modal_w / 2.0;
    let start_y = sh / 2.0 - 50.0;
    let (mx, my) = mouse_position();

    for (i, &cat) in candidates.iter().enumerate() {
        let y = start_y + i as f32 * 44.0;
        let label = format!("{}  {} {}", i + 1, cat.emoji(), cat.label());
        let hovered = mx >= modal_x + 20.0
            && mx <= modal_x + modal_w - 20.0
            && my >= y
            && my <= y + 38.0;

        let bg = if hovered {
            Color::new(0.12, 0.25, 0.35, 0.9)
        } else {
            Color::new(0.08, 0.12, 0.15, 0.6)
        };
        draw_rectangle(modal_x + 20.0, y, modal_w - 40.0, 38.0, bg);
        if hovered {
            draw_rectangle_lines(
                modal_x + 20.0, y, modal_w - 40.0, 38.0,
                1.0, Color::new(0.3, 0.5, 0.7, 0.8),
            );
        }
        draw_text(
            &label,
            modal_x + 30.0,
            y + 25.0,
            18.0,
            if hovered { WHITE } else { Color::new(0.7, 0.8, 0.9, 0.9) },
        );

        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            return InfluenceAction::CouncilPick(i);
        }
    }

    // Warning about override
    draw_text(
        "The mayor may override your choice (15% chance).",
        modal_x + 20.0,
        start_y + 150.0,
        13.0,
        Color::new(0.6, 0.5, 0.4, 0.6),
    );

    InfluenceAction::None
}

fn draw_audience_modal(
    input: &mut String,
    response: &Option<String>,
    waiting: bool,
) -> InfluenceAction {
    let llm_mode = crate::mayor::llm::api_available();
    let subtitle = if llm_mode {
        "Speak directly to the mayor (AI-powered):"
    } else {
        "Speak directly to the mayor:"
    };
    let (action, _) = draw_modal_frame("Direct Audience", subtitle);

    if action == InfluenceAction::CloseModal {
        return action;
    }

    let sw = screen_width();
    let sh = screen_height();
    let modal_w = 480.0;
    let modal_x = sw / 2.0 - modal_w / 2.0;
    let input_y = sh / 2.0 - 30.0;

    // Show response if we have one
    if let Some(resp) = response {
        draw_text(
            "The mayor responds:",
            modal_x + 20.0,
            input_y - 20.0,
            14.0,
            Color::new(0.5, 0.7, 0.5, 0.7),
        );

        // Response box
        draw_rectangle(
            modal_x + 20.0, input_y,
            modal_w - 40.0, 60.0,
            Color::new(0.1, 0.15, 0.1, 0.8),
        );

        // Word wrap response
        let max_chars = 55;
        let mut line_y = input_y + 18.0;
        let mut start = 0;
        while start < resp.len() && line_y < input_y + 55.0 {
            let end = (start + max_chars).min(resp.len());
            let slice = if end < resp.len() {
                if let Some(space) = resp[start..end].rfind(' ') {
                    &resp[start..start + space]
                } else {
                    &resp[start..end]
                }
            } else {
                &resp[start..end]
            };
            draw_text(slice, modal_x + 28.0, line_y, 14.0, Color::new(0.8, 0.9, 0.7, 1.0));
            line_y += 16.0;
            start += slice.len();
            if start < resp.len() && resp.as_bytes().get(start) == Some(&b' ') {
                start += 1;
            }
        }

        draw_text(
            "The mayor will consider your words. [ESC to close]",
            modal_x + 20.0,
            input_y + 80.0,
            13.0,
            Color::new(0.5, 0.6, 0.5, 0.6),
        );

        return InfluenceAction::None;
    }

    if waiting {
        draw_text(
            "Requesting audience with the mayor...",
            modal_x + 20.0,
            input_y + 20.0,
            16.0,
            Color::new(0.7, 0.8, 0.5, 0.8),
        );
        return InfluenceAction::None;
    }

    // Text input field
    draw_rectangle(
        modal_x + 20.0, input_y,
        modal_w - 40.0, 30.0,
        Color::new(0.12, 0.16, 0.12, 0.9),
    );
    draw_rectangle_lines(
        modal_x + 20.0, input_y,
        modal_w - 40.0, 30.0,
        1.0,
        Color::new(0.3, 0.5, 0.3, 0.6),
    );

    // Cursor blink
    let cursor = if (get_time() * 2.0) as u32 % 2 == 0 { "|" } else { "" };
    let display = format!("{}{}", input, cursor);
    draw_text(
        &display,
        modal_x + 26.0,
        input_y + 21.0,
        15.0,
        Color::new(0.8, 0.9, 0.8, 1.0),
    );

    // Handle text input
    while let Some(ch) = get_char_pressed() {
        if ch.is_ascii_graphic() || ch == ' ' {
            if input.len() < 200 {
                input.push(ch);
            }
        }
    }
    if is_key_pressed(KeyCode::Backspace) && !input.is_empty() {
        input.pop();
    }

    // Submit button
    let btn_y = input_y + 40.0;
    let (mx, my) = mouse_position();
    let btn_hover = mx >= modal_x + 20.0
        && mx <= modal_x + 140.0
        && my >= btn_y
        && my <= btn_y + 28.0;
    let can_submit = !input.is_empty();

    let bg = if !can_submit {
        Color::new(0.1, 0.1, 0.1, 0.4)
    } else if btn_hover {
        Color::new(0.2, 0.4, 0.2, 0.9)
    } else {
        Color::new(0.12, 0.25, 0.12, 0.8)
    };
    draw_rectangle(modal_x + 20.0, btn_y, 120.0, 28.0, bg);
    draw_text(
        "Send [ENTER]",
        modal_x + 28.0,
        btn_y + 19.0,
        14.0,
        if can_submit {
            Color::new(0.7, 1.0, 0.7, 1.0)
        } else {
            Color::new(0.4, 0.4, 0.4, 0.5)
        },
    );

    draw_text(
        "Try mentioning: parks, roads, growth, fire, money...",
        modal_x + 20.0,
        btn_y + 48.0,
        12.0,
        Color::new(0.4, 0.5, 0.4, 0.5),
    );

    if can_submit
        && (is_key_pressed(KeyCode::Enter)
            || (btn_hover && is_mouse_button_pressed(MouseButton::Left)))
    {
        return InfluenceAction::AudienceSubmit(input.clone());
    }

    InfluenceAction::None
}

fn draw_buy_ip_modal() -> InfluenceAction {
    let (action, _) = draw_modal_frame("Buy Influence Points", "Spend city funds to gain IP:");

    if action == InfluenceAction::CloseModal {
        return action;
    }

    let sw = screen_width();
    let sh = screen_height();
    let modal_w = 360.0;
    let modal_x = sw / 2.0 - modal_w / 2.0;
    let y = sh / 2.0 - 20.0;
    let (mx, my) = mouse_position();

    draw_text(
        "Cost: $5,000 per IP",
        modal_x + 20.0,
        y,
        16.0,
        Color::new(0.7, 0.8, 0.7, 0.9),
    );

    draw_text(
        "This drains the mayor's budget!",
        modal_x + 20.0,
        y + 22.0,
        13.0,
        Color::new(0.7, 0.5, 0.3, 0.7),
    );

    let btn_y = y + 40.0;
    let btn_hover = mx >= modal_x + 20.0
        && mx <= modal_x + 160.0
        && my >= btn_y
        && my <= btn_y + 30.0;

    let bg = if btn_hover {
        Color::new(0.2, 0.4, 0.2, 0.9)
    } else {
        Color::new(0.12, 0.25, 0.12, 0.8)
    };
    draw_rectangle(modal_x + 20.0, btn_y, 140.0, 30.0, bg);
    draw_rectangle_lines(
        modal_x + 20.0, btn_y, 140.0, 30.0,
        1.0, Color::new(0.3, 0.6, 0.3, 0.8),
    );
    draw_text(
        "Buy 1 IP",
        modal_x + 40.0,
        btn_y + 21.0,
        16.0,
        Color::new(0.7, 1.0, 0.7, 1.0),
    );

    if btn_hover && is_mouse_button_pressed(MouseButton::Left) {
        return InfluenceAction::BuyIPConfirm;
    }

    InfluenceAction::None
}

// ===== HELPERS =====

/// Draw a standard modal frame (darkened background, centered box, close button).
/// Returns (CloseModal if ESC pressed, modal_y for content positioning).
fn draw_modal_frame(title: &str, subtitle: &str) -> (InfluenceAction, f32) {
    let sw = screen_width();
    let sh = screen_height();

    // Darkened background
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));

    // Modal box
    let modal_w = 480.0;
    let modal_h = 350.0;
    let modal_x = sw / 2.0 - modal_w / 2.0;
    let modal_y = sh / 2.0 - modal_h / 2.0;

    draw_rectangle(modal_x, modal_y, modal_w, modal_h, Color::new(0.06, 0.09, 0.06, 0.95));
    draw_rectangle_lines(modal_x, modal_y, modal_w, modal_h, 2.0, Color::new(0.3, 0.5, 0.3, 0.8));

    // Title
    draw_text(
        title,
        modal_x + 20.0,
        modal_y + 30.0,
        24.0,
        Color::new(0.7, 0.95, 0.7, 1.0),
    );

    // Subtitle
    draw_text(
        subtitle,
        modal_x + 20.0,
        modal_y + 52.0,
        14.0,
        Color::new(0.5, 0.6, 0.5, 0.7),
    );

    // Close hint
    draw_text(
        "[ESC] Close",
        modal_x + modal_w - 90.0,
        modal_y + 30.0,
        13.0,
        Color::new(0.5, 0.5, 0.5, 0.6),
    );

    // ESC to close
    if is_key_pressed(KeyCode::Escape) {
        return (InfluenceAction::CloseModal, modal_y);
    }

    (InfluenceAction::None, modal_y)
}

fn draw_small_btn(
    x: f32, y: f32, w: f32, h: f32,
    label: &str, color: Color,
    mx: f32, my: f32,
) -> bool {
    let hovered = mx >= x && mx <= x + w && my >= y && my <= y + h;
    let bg = if hovered {
        Color::new(color.r + 0.1, color.g + 0.1, color.b + 0.1, 0.9)
    } else {
        color
    };
    draw_rectangle(x, y, w, h, bg);
    let lw = measure_text(label, None, 11, 1.0).width;
    draw_text(label, x + w / 2.0 - lw / 2.0, y + 14.0, 11.0, WHITE);

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_tier_btn(
    x: f32, y: f32, w: f32, h: f32,
    label: &str, cost: u32,
    state: &InfluenceState,
    color: Color,
    mx: f32, my: f32,
) -> bool {
    let affordable = state.can_afford(cost);
    let hovered = mx >= x && mx <= x + w && my >= y && my <= y + h;

    let bg = if !affordable {
        Color::new(0.1, 0.1, 0.1, 0.4)
    } else if hovered {
        Color::new(color.r + 0.1, color.g + 0.1, color.b + 0.1, 0.95)
    } else {
        color
    };
    draw_rectangle(x, y, w, h, bg);
    if affordable {
        draw_rectangle_lines(x, y, w, h, 1.0, Color::new(color.r + 0.2, color.g + 0.2, color.b + 0.2, 0.6));
    }

    let text_color = if affordable {
        WHITE
    } else {
        Color::new(0.4, 0.4, 0.4, 0.5)
    };
    let lw = measure_text(label, None, 12, 1.0).width;
    draw_text(label, x + w / 2.0 - lw / 2.0, y + 17.0, 12.0, text_color);

    affordable && hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_disaster_btn(
    state: &InfluenceState,
    x: f32, y: f32, w: f32, h: f32,
    mx: f32, my: f32,
) -> bool {
    let on_cooldown = state.disaster_cooldown > 0.0;
    let maxed_out = state.disasters_this_year >= 2;
    let available = !on_cooldown && !maxed_out;

    let hovered = mx >= x && mx <= x + w && my >= y && my <= y + h;

    let bg = if !available {
        Color::new(0.2, 0.15, 0.15, 0.5)
    } else if hovered {
        Color::new(0.6, 0.2, 0.1, 0.9)
    } else {
        Color::new(0.4, 0.15, 0.08, 0.8)
    };

    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(
        x, y, w, h, 1.0,
        if available {
            Color::new(0.8, 0.3, 0.1, 0.8)
        } else {
            Color::new(0.3, 0.2, 0.15, 0.4)
        },
    );

    let label = if on_cooldown {
        format!("FIRE (cooldown {:.0}s)", state.disaster_cooldown)
    } else if maxed_out {
        "FIRE (max 2/year)".to_string()
    } else {
        "FIRE [+2 IP]".to_string()
    };

    let lw = measure_text(&label, None, 14, 1.0).width;
    draw_text(
        &label,
        x + w / 2.0 - lw / 2.0,
        y + 19.0,
        14.0,
        if available {
            Color::new(1.0, 0.6, 0.3, 1.0)
        } else {
            Color::new(0.5, 0.4, 0.3, 0.5)
        },
    );

    available && hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Draw the in-game speed slider at the bottom of the screen.
pub fn draw_speed_slider(current_idx: usize) -> Option<usize> {
    let sw = screen_width();
    let sh = screen_height();

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
