use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::mayor::personality::MayorPersonality;
use super::{ActionCategory, MayorResponse};

/// Evaluate a suggestion box request.
/// Compliance probability = 0.3 + 0.5 × alignment (+ boost if active).
pub fn evaluate_suggestion(
    action: ActionCategory,
    personality: &MayorPersonality,
    has_boost: bool,
    rng: &mut SmallRng,
) -> MayorResponse {
    let alignment = action.alignment(personality);
    let base_prob = 0.3 + 0.5 * alignment;
    let prob = if has_boost { (base_prob + 0.25).min(0.95) } else { base_prob };

    let roll = rng.gen::<f32>();

    if roll < prob {
        // Comply
        let text = comply_text(action, personality, rng);
        MayorResponse::Comply(action, text)
    } else if roll < prob + 0.3 {
        // Ignore
        let text = ignore_text(action, personality, rng);
        MayorResponse::Ignore(text)
    } else {
        // Argue
        let text = argue_text(action, personality, rng);
        MayorResponse::Argue(text)
    }
}

fn comply_text(action: ActionCategory, _p: &MayorPersonality, rng: &mut SmallRng) -> String {
    let options = match action {
        ActionCategory::BuildPark => &[
            "Fine, I'll build your park. Happy?",
            "A park it is. The people will appreciate this.",
            "Green space approved. You have good instincts.",
        ][..],
        ActionCategory::ZoneResidential => &[
            "More homes? Already on it.",
            "Residential zoning approved. People need places to live.",
            "New housing going up. Good suggestion.",
        ],
        ActionCategory::ZoneCommercial => &[
            "Commercial district expansion approved.",
            "Shops and offices. The economy needs this.",
            "Commercial zoning it is. Smart thinking.",
        ],
        ActionCategory::ZoneIndustrial => &[
            "Industrial zone approved. Revenue incoming.",
            "Factories going up. This will boost the treasury.",
            "Industrial expansion. I was thinking the same thing.",
        ],
        ActionCategory::ExtendPower => &[
            "Power grid extension approved. Lights for everyone.",
            "Running more power lines. Essential infrastructure.",
            "Extending the grid. Good call.",
        ],
        ActionCategory::ExtendWater => &[
            "Water supply extension approved.",
            "More water infrastructure. The people need this.",
            "Extending water mains. Clean water for all.",
        ],
        ActionCategory::BuildRoads => &[
            "New roads approved. Access drives growth.",
            "Laying asphalt. Every neighborhood deserves a road.",
            "Road expansion. Connectivity is key.",
        ],
    };
    options[rng.gen_range(0..options.len())].to_string()
}

fn ignore_text(_action: ActionCategory, _p: &MayorPersonality, rng: &mut SmallRng) -> String {
    let options = &[
        "Not a priority right now.",
        "I'll consider it. Later.",
        "The suggestion box is noted. Moving on.",
        "Interesting idea. Filed for future review.",
        "I have other plans at the moment.",
    ];
    options[rng.gen_range(0..options.len())].to_string()
}

fn argue_text(action: ActionCategory, p: &MayorPersonality, rng: &mut SmallRng) -> String {
    let options = match action {
        ActionCategory::BuildPark if p.green_affinity < 0.3 => &[
            "Parks are wasted land. We need industry.",
            "Green space? In this economy?",
            "Every square meter of park is revenue we're not earning.",
        ][..],
        ActionCategory::ZoneIndustrial if p.green_affinity > 0.7 => &[
            "More factories? Over my dead body.",
            "Industrial pollution is already too high.",
            "The people don't want more smokestacks.",
        ],
        ActionCategory::ZoneResidential if p.growth_aggression < 0.4 => &[
            "Slow down. We can't just sprawl everywhere.",
            "Quality over quantity. We're not ready for more homes.",
            "Expansion without infrastructure is reckless.",
        ],
        _ => &[
            "I disagree. My city, my rules.",
            "That's not how I'd do it.",
            "You're not seeing the full picture here.",
            "I appreciate the input, but no.",
        ],
    };
    options[rng.gen_range(0..options.len())].to_string()
}
