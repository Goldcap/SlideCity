use crate::sim::stats::CityStats;

/// Music track identifiers mapped to city mood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackId {
    EmptyLand,
    FirstStreets,
    GrowingCity,
    BoomTown,
    Disaster,
    Recovery,
    Decline,
    Monument, // One-shot sting, not looped
}

impl TrackId {
    /// Local audio filename (in assets/audio/).
    pub fn filename(self) -> &'static str {
        match self {
            TrackId::EmptyLand => "empty_land.ogg",
            TrackId::FirstStreets => "first_streets.ogg",
            TrackId::GrowingCity => "growing_city.ogg",
            TrackId::BoomTown => "boom_town.ogg",
            TrackId::Disaster => "disaster.ogg",
            TrackId::Recovery => "recovery.ogg",
            TrackId::Decline => "decline.ogg",
            TrackId::Monument => "monument.ogg",
        }
    }

    /// Whether this track should loop.
    pub fn looped(self) -> bool {
        self != TrackId::Monument
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            TrackId::EmptyLand => "Empty Land",
            TrackId::FirstStreets => "First Streets",
            TrackId::GrowingCity => "Growing City",
            TrackId::BoomTown => "Boom Town",
            TrackId::Disaster => "Disaster",
            TrackId::Recovery => "Recovery",
            TrackId::Decline => "Decline",
            TrackId::Monument => "Monument",
        }
    }
}

/// Select the appropriate track based on city state.
pub fn select_track(stats: &CityStats) -> TrackId {
    if stats.fire_count > 2 {
        return TrackId::Disaster;
    }
    if stats.happiness < 0.30 {
        return TrackId::Decline;
    }
    if stats.population == 0 {
        return TrackId::EmptyLand;
    }
    if stats.population < 60 {
        return TrackId::FirstStreets;
    }
    if stats.population < 350 {
        return TrackId::GrowingCity;
    }
    if stats.happiness < 0.50 {
        return TrackId::Recovery;
    }
    TrackId::BoomTown
}
