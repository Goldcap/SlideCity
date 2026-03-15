use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::mayor::personality::MayorPersonality;
use crate::sim::stats::CityStats;
use super::{ActionCategory, MayorResponse};

/// Generate 3 candidate actions for the council vote based on city assessment.
pub fn generate_candidates(
    personality: &MayorPersonality,
    stats: &CityStats,
    rng: &mut SmallRng,
) -> [ActionCategory; 3] {
    // Score each action based on city needs + personality
    let mut scored: Vec<(ActionCategory, f32)> = ActionCategory::ALL
        .iter()
        .map(|&a| {
            let need = city_need_score(a, stats);
            let pref = a.alignment(personality);
            let score = need * 0.6 + pref * 0.4 + rng.gen::<f32>() * 0.2;
            (a, score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Return top 3
    [scored[0].0, scored[1].0, scored[2].0]
}

/// Execute a council vote. The mayor executes the player's choice,
/// but has a 15% chance of overriding with their preferred option.
pub fn execute_vote(
    player_choice: ActionCategory,
    candidates: &[ActionCategory; 3],
    personality: &MayorPersonality,
    has_boost: bool,
    rng: &mut SmallRng,
) -> MayorResponse {
    let override_chance = if has_boost { 0.05 } else { 0.15 };

    if rng.gen::<f32>() < override_chance {
        // Mayor overrides — pick their most preferred candidate
        let preferred = candidates
            .iter()
            .max_by(|a, b| {
                a.alignment(personality)
                    .partial_cmp(&b.alignment(personality))
                    .unwrap()
            })
            .copied()
            .unwrap_or(player_choice);

        if preferred != player_choice {
            let text = override_text(preferred, rng);
            return MayorResponse::Override(preferred, text);
        }
    }

    // Mayor complies (possibly reluctantly)
    let alignment = player_choice.alignment(personality);
    let text = if alignment > 0.6 {
        council_comply_eager(player_choice, rng)
    } else if alignment > 0.3 {
        council_comply_neutral(player_choice, rng)
    } else {
        council_comply_reluctant(player_choice, rng)
    };

    MayorResponse::Comply(player_choice, text)
}

/// Score how much the city needs a particular action (0.0 - 1.0).
fn city_need_score(action: ActionCategory, stats: &CityStats) -> f32 {
    match action {
        ActionCategory::BuildPark => {
            if stats.happiness < 0.4 { 0.9 }
            else if stats.happiness < 0.6 { 0.6 }
            else { 0.2 }
        }
        ActionCategory::ZoneResidential => {
            if stats.res_count < 20 { 0.8 }
            else if stats.population < 200 { 0.5 }
            else { 0.3 }
        }
        ActionCategory::ZoneCommercial => {
            let ratio = if stats.com_count > 0 {
                stats.res_count as f32 / stats.com_count as f32
            } else {
                999.0
            };
            if ratio > 6.0 { 0.8 } else if ratio > 4.0 { 0.5 } else { 0.2 }
        }
        ActionCategory::ZoneIndustrial => {
            let ratio = if stats.ind_count > 0 {
                stats.res_count as f32 / stats.ind_count as f32
            } else {
                999.0
            };
            if ratio > 8.0 { 0.7 } else if ratio > 5.0 { 0.4 } else { 0.2 }
        }
        ActionCategory::ExtendPower => {
            if stats.power_coverage < 0.5 { 0.9 }
            else if stats.power_coverage < 0.7 { 0.5 }
            else { 0.15 }
        }
        ActionCategory::ExtendWater => {
            if stats.water_coverage < 0.5 { 0.9 }
            else if stats.water_coverage < 0.7 { 0.5 }
            else { 0.15 }
        }
        ActionCategory::BuildRoads => {
            if stats.road_count < 30 { 0.7 }
            else if stats.total_developed > stats.road_count * 5 { 0.5 }
            else { 0.2 }
        }
    }
}

fn override_text(action: ActionCategory, rng: &mut SmallRng) -> String {
    let base = &[
        "I know what this city needs.",
        "Council voted, but I'm overriding.",
        "Democracy is nice, but I'm the mayor.",
        "I appreciate the vote, but I'm going with my gut.",
    ];
    format!(
        "{} Going with: {}",
        base[rng.gen_range(0..base.len())],
        action.label()
    )
}

fn council_comply_eager(action: ActionCategory, rng: &mut SmallRng) -> String {
    let options = &[
        "Great choice. I was hoping you'd pick that.",
        "Now we're talking. Executing immediately.",
        "Exactly what I would have chosen. Let's do it.",
    ];
    format!(
        "{} [{}]",
        options[rng.gen_range(0..options.len())],
        action.label()
    )
}

fn council_comply_neutral(action: ActionCategory, rng: &mut SmallRng) -> String {
    let options = &[
        "The council has spoken. Making it happen.",
        "Not my first choice, but I'll execute it.",
        "Reasonable enough. Proceeding.",
    ];
    format!(
        "{} [{}]",
        options[rng.gen_range(0..options.len())],
        action.label()
    )
}

fn council_comply_reluctant(action: ActionCategory, rng: &mut SmallRng) -> String {
    let options = &[
        "If you insist. Don't blame me if this backfires.",
        "Against my better judgment... executing.",
        "The council wants what it wants. Fine.",
    ];
    format!(
        "{} [{}]",
        options[rng.gen_range(0..options.len())],
        action.label()
    )
}
