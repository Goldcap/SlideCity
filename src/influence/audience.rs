use ::rand::rngs::SmallRng;
use ::rand::Rng;

use crate::mayor::personality::MayorPersonality;

/// Result of a Direct Audience interaction.
#[derive(Clone, Debug)]
pub struct AudienceResult {
    pub response: String,
    pub compliance_boost: u32,
}

/// Process a direct audience request with a scripted personality response.
/// (LLM integration deferred to Phase 8 — this provides a rich fallback.)
pub fn process_audience(
    player_message: &str,
    personality: &MayorPersonality,
    rng: &mut SmallRng,
) -> AudienceResult {
    let response = generate_response(player_message, personality, rng);

    AudienceResult {
        response,
        compliance_boost: rng.gen_range(2..=3),
    }
}

fn generate_response(message: &str, p: &MayorPersonality, rng: &mut SmallRng) -> String {
    let msg_lower = message.to_lowercase();

    // Topic detection — scan for keywords
    if msg_lower.contains("park") || msg_lower.contains("green") || msg_lower.contains("tree") {
        return green_response(p, rng);
    }
    if msg_lower.contains("industry") || msg_lower.contains("factory") || msg_lower.contains("industrial") {
        return industry_response(p, rng);
    }
    if msg_lower.contains("road") || msg_lower.contains("transport") || msg_lower.contains("connect") {
        return roads_response(p, rng);
    }
    if msg_lower.contains("power") || msg_lower.contains("electric") || msg_lower.contains("energy") {
        return power_response(p, rng);
    }
    if msg_lower.contains("water") || msg_lower.contains("pipe") || msg_lower.contains("supply") {
        return water_response(p, rng);
    }
    if msg_lower.contains("fire") || msg_lower.contains("disaster") || msg_lower.contains("emergency") {
        return disaster_response(p, rng);
    }
    if msg_lower.contains("happy") || msg_lower.contains("people") || msg_lower.contains("citizen") {
        return happiness_response(p, rng);
    }
    if msg_lower.contains("money") || msg_lower.contains("fund") || msg_lower.contains("budget") || msg_lower.contains("tax") {
        return money_response(p, rng);
    }
    if msg_lower.contains("grow") || msg_lower.contains("expand") || msg_lower.contains("build") {
        return growth_response(p, rng);
    }

    // Generic response
    generic_response(p, rng)
}

fn green_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.green_affinity > 0.6 {
        pick(rng, &[
            "You're speaking my language. Parks are the soul of a city.",
            "I've been thinking the same thing. More green space coming.",
            "A city without nature is just concrete. I'll prioritize this.",
        ])
    } else {
        pick(rng, &[
            "Parks are nice, but they don't pay taxes.",
            "I hear you, but we have bigger priorities right now.",
            "Green space... maybe. Once the economy's stable.",
        ])
    }
}

fn industry_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.industrial_bias > 0.6 {
        pick(rng, &[
            "Industry is the backbone of this city. I'll expand it.",
            "Factories mean jobs. Jobs mean growth. Consider it done.",
            "You understand economics. More industrial zones coming.",
        ])
    } else {
        pick(rng, &[
            "More factories? The pollution is already a concern.",
            "Industrial growth needs to be balanced with livability.",
            "I'll consider it, but not at the expense of the people.",
        ])
    }
}

fn roads_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.growth_aggression > 0.5 {
        pick(rng, &[
            "Connectivity is everything. I'll extend the road network.",
            "Roads first, then growth follows. Sound strategy.",
            "You can't have a city without roads. Approved.",
        ])
    } else {
        pick(rng, &[
            "More roads invite more sprawl. We need to be careful.",
            "I'd rather improve what we have than spread further.",
            "Roads are expensive. Let's focus on the core first.",
        ])
    }
}

fn power_response(_p: &MayorPersonality, rng: &mut SmallRng) -> String {
    pick(rng, &[
        "Power infrastructure is always a priority. I'll look into it.",
        "Everyone deserves electricity. Extending the grid.",
        "Dark homes are unhappy homes. Power expansion noted.",
    ])
}

fn water_response(_p: &MayorPersonality, rng: &mut SmallRng) -> String {
    pick(rng, &[
        "Clean water is a right, not a privilege. I'll extend the supply.",
        "Water infrastructure is essential. Noted.",
        "No city survives without water. Prioritizing this.",
    ])
}

fn disaster_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.panic_threshold > 0.6 {
        pick(rng, &[
            "Don't... don't remind me about disasters. I'm doing my best.",
            "Fire prevention is always on my mind. Always.",
            "Every disaster ages me ten years. I'm working on preparedness.",
        ])
    } else {
        pick(rng, &[
            "Disasters are part of city life. We recover and rebuild.",
            "I've planned for contingencies. We'll be fine.",
            "Fire? We'll handle it. This city is resilient.",
        ])
    }
}

fn happiness_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.green_affinity > 0.5 {
        pick(rng, &[
            "Happy citizens are productive citizens. More parks and services coming.",
            "The people's wellbeing is my top priority.",
            "Happiness drives everything. I'll focus on quality of life.",
        ])
    } else {
        pick(rng, &[
            "Happiness follows prosperity. Build the economy first.",
            "The people will be happier when they have jobs.",
            "I care about the people, but growth comes first.",
        ])
    }
}

fn money_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.growth_aggression > 0.6 {
        pick(rng, &[
            "Money flows from growth. Build more, earn more.",
            "The treasury needs feeding. More zones, more taxes.",
            "Fiscal policy: expand. Revenue follows development.",
        ])
    } else {
        pick(rng, &[
            "Budget management is about balance, not just earning.",
            "I'm watching every dollar. We'll spend wisely.",
            "The treasury is my responsibility. I'll be prudent.",
        ])
    }
}

fn growth_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    if p.growth_aggression > 0.6 {
        pick(rng, &[
            "Growth is the mission. Full speed ahead.",
            "Build, build, build. That's the plan.",
            "This city's potential is unlimited. Let's expand.",
        ])
    } else {
        pick(rng, &[
            "Growth must be sustainable. Rushing leads to problems.",
            "Quality over quantity. But yes, we'll grow.",
            "Measured growth is smart growth. Patience.",
        ])
    }
}

fn generic_response(p: &MayorPersonality, rng: &mut SmallRng) -> String {
    let greetings = &[
        format!("As {}, I'll take that under advisement.", p.name),
        format!("I hear you. {} is always listening to the people.", p.name),
        "Interesting perspective. I'll factor that into my decisions.".to_string(),
        "You have my attention. What specifically would you like me to prioritize?".to_string(),
        "The people's voice matters. I'll adjust my approach.".to_string(),
        format!("Thank you for the audience. {} will consider this carefully.", p.name),
    ];
    greetings[rng.gen_range(0..greetings.len())].clone()
}

fn pick(rng: &mut SmallRng, options: &[&str]) -> String {
    options[rng.gen_range(0..options.len())].to_string()
}
