use macroquad::prelude::*;

use crate::sim::stats::CityStats;
use crate::renderer::lighting::DayNightCycle;
use crate::mayor::MayorPhase;

/// Draw the top HUD bar with city stats.
pub fn draw_hud(
    stats: &CityStats,
    funds: i64,
    year: u64,
    season: &str,
    speed: f32,
    day_night: &DayNightCycle,
    phase: MayorPhase,
    music_label: &str,
) {
    let sw = screen_width();

    // Background bar
    draw_rectangle(0.0, 0.0, sw, 52.0, Color::new(0.02, 0.04, 0.02, 0.85));
    draw_line(0.0, 52.0, sw, 52.0, 1.0, Color::new(0.2, 0.35, 0.2, 0.5));

    // Row 1: core stats
    let mut x = 10.0;
    let y1 = 20.0;
    let y2 = 42.0;

    // Population
    let pop_color = if stats.population > 500 {
        Color::new(0.5, 1.0, 0.5, 1.0)
    } else if stats.population > 100 {
        Color::new(0.7, 0.9, 0.7, 1.0)
    } else {
        WHITE
    };
    let pop_text = format!("Pop: {}", stats.population);
    draw_text(&pop_text, x, y1, 18.0, pop_color);
    x += measure_text(&pop_text, None, 18, 1.0).width + 20.0;

    // R/C/I counts
    let rci = format!("R:{} C:{} I:{}", stats.res_count, stats.com_count, stats.ind_count);
    draw_text(&rci, x, y1, 16.0, Color::new(0.7, 0.8, 0.7, 0.9));
    x += measure_text(&rci, None, 16, 1.0).width + 20.0;

    // Funds
    let funds_color = if funds < 5000 {
        Color::new(1.0, 0.4, 0.3, 1.0)
    } else if funds < 15000 {
        Color::new(1.0, 0.8, 0.3, 1.0)
    } else {
        Color::new(0.5, 0.9, 0.5, 1.0)
    };
    let funds_text = format!("${}", funds);
    draw_text(&funds_text, x, y1, 18.0, funds_color);
    x += measure_text(&funds_text, None, 18, 1.0).width + 20.0;

    // Year + Season
    let time_text = format!("Year {}, {}", year, season);
    draw_text(&time_text, x, y1, 16.0, Color::new(0.8, 0.85, 0.7, 1.0));

    // Row 2: secondary stats
    x = 10.0;

    // Happiness bar
    let happy_pct = (stats.happiness * 100.0) as u32;
    let happy_color = if stats.happiness > 0.65 {
        Color::new(0.4, 0.9, 0.4, 1.0)
    } else if stats.happiness > 0.4 {
        Color::new(0.9, 0.8, 0.3, 1.0)
    } else {
        Color::new(0.9, 0.3, 0.3, 1.0)
    };
    draw_text(&format!("Happy: {}%", happy_pct), x, y2, 15.0, happy_color);
    // Mini bar
    let bar_x = x + 80.0;
    draw_rectangle(bar_x, y2 - 10.0, 60.0, 8.0, Color::new(0.15, 0.15, 0.15, 0.8));
    draw_rectangle(bar_x, y2 - 10.0, 60.0 * stats.happiness, 8.0, happy_color);
    x = bar_x + 70.0;

    // Power coverage
    let pwr_pct = (stats.power_coverage * 100.0) as u32;
    let pwr_color = if stats.power_coverage > 0.7 {
        Color::new(0.9, 0.7, 0.2, 1.0)
    } else {
        Color::new(0.8, 0.4, 0.2, 1.0)
    };
    draw_text(&format!("Pwr: {}%", pwr_pct), x, y2, 15.0, pwr_color);
    x += 80.0;

    // Water coverage
    let wtr_pct = (stats.water_coverage * 100.0) as u32;
    let wtr_color = if stats.water_coverage > 0.7 {
        Color::new(0.3, 0.6, 1.0, 1.0)
    } else {
        Color::new(0.5, 0.3, 0.8, 1.0)
    };
    draw_text(&format!("Wtr: {}%", wtr_pct), x, y2, 15.0, wtr_color);
    x += 80.0;

    // Fire count
    if stats.fire_count > 0 {
        let fire_text = format!("Fire: {}", stats.fire_count);
        draw_text(&fire_text, x, y2, 15.0, Color::new(1.0, 0.3, 0.1, 1.0));
        x += measure_text(&fire_text, None, 15, 1.0).width + 15.0;
    }

    // Speed
    draw_text(
        &format!("{}x", speed as u32),
        x,
        y2,
        15.0,
        Color::new(0.5, 0.7, 0.5, 0.8),
    );
    x += 35.0;

    // Day/night phase
    draw_text(
        day_night.phase_label(),
        x,
        y2,
        14.0,
        Color::new(0.5, 0.5, 0.6, 0.6),
    );
    x += measure_text(day_night.phase_label(), None, 14, 1.0).width + 15.0;

    // Mayor phase
    draw_text(
        &format!("{:?}", phase),
        x,
        y2,
        14.0,
        Color::new(0.5, 0.6, 0.5, 0.6),
    );
    x += 80.0;

    // Music mood
    draw_text(
        &format!("Music: {}", music_label),
        x,
        y2,
        14.0,
        Color::new(0.4, 0.5, 0.5, 0.5),
    );
}
