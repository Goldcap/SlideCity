use ::rand::rngs::SmallRng;
use ::rand::Rng;
use serde::{Deserialize, Serialize};

/// A single entry in the mayor's log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub year: u32,
    pub season: String,
    pub text: String,
    pub emoji: String,
}

/// The mayor's scrolling thought log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MayorLog {
    pub entries: Vec<LogEntry>,
}

impl MayorLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, year: u32, season: &str, emoji: &str, text: String) {
        self.entries.push(LogEntry {
            year,
            season: season.to_string(),
            text,
            emoji: emoji.to_string(),
        });
        // Keep last 20 entries in memory, UI shows last 7
        if self.entries.len() > 20 {
            self.entries.remove(0);
        }
    }

    /// Get the last N entries for display (newest first).
    pub fn last_n(&self, n: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(n).collect()
    }
}

/// Narration context for selecting appropriate strings.
#[derive(Clone, Copy, Debug)]
pub enum NarrationContext {
    // Founding
    RoadSpine,
    FirstResidential,
    PowerPlantBuilt,
    PowerLineRun,
    WaterTowerBuilt,
    WaterMainRun,

    // Growth - optimistic
    PopulationGrowing,
    CommercialExpansion,
    RoadExpansion,
    FundsHealthy,

    // Growth - stressed
    PowerCoverageLow,
    WaterUrgent,
    RatioImbalance,
    FirstPark,

    // Maturity - proud
    CityAlive,
    MonumentApproved,
    DensityManagement,

    // Maturity - troubled
    FireResponse,
    PollutionProblem,
    FundsTight,

    // Disaster
    FirePanic,
    FireWatching,
    Rebuilding,

    // Evolution
    Gentrification,
    IndustrialClearing,
    Retirement,
    NewMayor,

    // General actions
    ResidentialPlaced,
    CommercialPlaced,
    IndustrialPlaced,
    ParkPlaced,
    PowerExtended,
    WaterExtended,
    RubbleCleared,
}

/// Pick a narration string for a given context.
pub fn narrate(ctx: NarrationContext, rng: &mut SmallRng) -> &'static str {
    let options = match ctx {
        // Founding
        NarrationContext::RoadSpine => &[
            "Road spine complete. The city starts here.",
            "First roads laid. Every great city begins with a path.",
            "Backbone's in. Time to build something worth driving to.",
        ][..],
        NarrationContext::FirstResidential => &[
            "Dropping the first residential zone. Welcome home, citizens.",
            "First homes going up. Population: hopeful.",
            "Residential zone seeded. Small beginnings.",
        ],
        NarrationContext::PowerPlantBuilt => &[
            "Power plant online. Let there be light.",
            "Power plant's up. Running lines now.",
            "Electricity. The lifeblood of progress.",
        ],
        NarrationContext::PowerLineRun => &[
            "Running power lines toward the neighborhood.",
            "Power lines extending east. Coverage improving.",
            "Wiring up the grid. Every home deserves power.",
        ],
        NarrationContext::WaterTowerBuilt => &[
            "Water tower up. This city is starting to breathe.",
            "Water infrastructure online. Essential.",
            "Fresh water secured. The people will thrive.",
        ],
        NarrationContext::WaterMainRun => &[
            "Water mains heading toward the residential zone.",
            "Extending water supply. Coverage is key.",
            "Laying pipe. Not glamorous, but necessary.",
        ],

        // Growth - optimistic
        NarrationContext::PopulationGrowing => &[
            "Population's growing fast. Time to zone more residential.",
            "People keep coming. We need more homes.",
            "Growth curve's healthy. Expanding housing.",
            "New residents arriving daily. The city calls.",
        ],
        NarrationContext::CommercialExpansion => &[
            "The commercial district needs room to expand. Making it happen.",
            "Shops and offices going up. Economy's warming.",
            "Commercial zone approved. The people need places to spend.",
            "R:C ratio's off. Dropping a commercial block.",
        ],
        NarrationContext::RoadExpansion => &[
            "Funds looking healthy. Let's lay more road.",
            "Extending the road network. Growth demands access.",
            "New roads connecting the outer zones.",
        ],
        NarrationContext::FundsHealthy => &[
            "Treasury's looking good. Time to invest.",
            "Surplus in the budget. Let's build something.",
            "Money's flowing. This is when we expand.",
        ],

        // Growth - stressed
        NarrationContext::PowerCoverageLow => &[
            "Power coverage is dropping. Need another line run.",
            "Too many homes, not enough power. Fix this now.",
            "Dark zones appearing. Extending the grid.",
        ],
        NarrationContext::WaterUrgent => &[
            "The people need water. This is urgent.",
            "Water coverage critical. Extending mains.",
            "Dry zones growing. Water infrastructure priority one.",
        ],
        NarrationContext::RatioImbalance => &[
            "R:C ratio's way off. Dropping a commercial block downtown.",
            "Too many homes, not enough shops. Zoning commercial.",
            "The economy needs balance. Commercial zoning time.",
        ],
        NarrationContext::FirstPark => &[
            "Happiness dipping. Time for the city's first park.",
            "Green space. Every city needs it. Going in now.",
            "The people need somewhere to breathe. Park approved.",
        ],

        // Maturity - proud
        NarrationContext::CityAlive => &[
            "Look at what we've built. This city is alive.",
            "Standing back for a moment. We did this.",
            "Every light in every window — that's someone's home.",
            "The skyline's filling in. Beautiful.",
        ],
        NarrationContext::MonumentApproved => &[
            "Monument approved. The people deserve something permanent.",
            "Five hundred souls. Time for something grand.",
            "This city has earned its monument.",
            "Breaking ground on the monument. A landmark for the ages.",
        ],
        NarrationContext::DensityManagement => &[
            "Density's peaking downtown. Time for green space.",
            "Managing growth carefully now. Quality over quantity.",
            "The core is dense. Expanding outward.",
        ],

        // Maturity - troubled
        NarrationContext::FireResponse => &[
            "Fire on the east block. Nothing to do but wait and rebuild.",
            "The fire department's stretched thin. Hold the line.",
            "Watching it burn. We'll come back stronger.",
        ],
        NarrationContext::PollutionProblem => &[
            "Industrial pollution is wrecking happiness. Park buffer going in.",
            "The factories are too close to homes. Need green space between.",
            "Pollution complaints. Building a park barrier.",
        ],
        NarrationContext::FundsTight => &[
            "Funds tight. Holding expansion until taxes recover.",
            "Budget's thin. Essential spending only.",
            "Tightening the belt. No new projects until revenue improves.",
        ],

        // Disaster
        NarrationContext::FirePanic => &[
            "Fire. Oh no. Fire.",
            "FIRE! Multiple blocks affected!",
            "The alarm bells are ringing. This is bad.",
        ],
        NarrationContext::FireWatching => &[
            "The east side is burning. I'm watching it happen.",
            "Flames spreading. Hold your breath.",
            "Nothing to do but watch and plan the rebuild.",
        ],
        NarrationContext::Rebuilding => &[
            "We'll rebuild. We always rebuild.",
            "Rubble clearing. New foundations coming.",
            "From the ashes, we build again.",
        ],

        // Evolution
        NarrationContext::Gentrification => &[
            "Gentrification wave rolling through the factory district.",
            "The old warehouses are becoming offices. Progress.",
            "Industrial zones converting to commercial. The city evolves.",
        ],
        NarrationContext::IndustrialClearing => &[
            "Old industrial zone's clearing out. Commercial moving in.",
            "Factories closing, storefronts opening. Change is constant.",
            "The industrial era gives way. Something new grows.",
        ],
        NarrationContext::Retirement => &[
            "My time is done. This city shaped me as much as I shaped it.",
            "Hanging up the hat. It's been an honor.",
            "Time to pass the torch. This city deserves fresh eyes.",
        ],
        NarrationContext::NewMayor => &[
            "New mayor in town. Ready to shake things up.",
            "Fresh perspective, same great city. Let's go.",
            "The previous mayor left big shoes. I'll fill them my way.",
        ],

        // General actions
        NarrationContext::ResidentialPlaced => &[
            "New residential zone seeded. Homes incoming.",
            "Zoning residential. The city grows.",
            "More homes for more people.",
        ],
        NarrationContext::CommercialPlaced => &[
            "Commercial zone approved. Business is good.",
            "New shops going up. The economy expands.",
            "Commercial zoning. Jobs and commerce.",
        ],
        NarrationContext::IndustrialPlaced => &[
            "Industrial zone approved. Production begins.",
            "Factories going up. Revenue incoming.",
            "Industrial expansion. The engine of growth.",
        ],
        NarrationContext::ParkPlaced => &[
            "New park. The city breathes easier.",
            "Green space approved. Happiness rising.",
            "Park going in. Worth every penny.",
        ],
        NarrationContext::PowerExtended => &[
            "Power lines extended. More homes lit up.",
            "Extending the grid. Coverage improving.",
            "Power reaching new zones.",
        ],
        NarrationContext::WaterExtended => &[
            "Water mains extended. Clean water for all.",
            "Extending water supply. Essential service.",
            "Water reaching the outer zones.",
        ],
        NarrationContext::RubbleCleared => &[
            "Clearing rubble. Making room for renewal.",
            "Demolition complete. Fresh start.",
            "The rubble's gone. What do we build next?",
        ],
    };

    options[rng.gen_range(0..options.len())]
}

/// Get season name from tick count.
pub fn season_name(tick_count: u64, ticks_per_season: u32) -> &'static str {
    let season_idx = (tick_count / ticks_per_season as u64) % 4;
    match season_idx {
        0 => "Spring",
        1 => "Summer",
        2 => "Fall",
        _ => "Winter",
    }
}
