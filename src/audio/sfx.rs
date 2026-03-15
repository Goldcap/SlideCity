use std::collections::HashMap;
use macroquad::audio::{load_sound, play_sound, Sound, PlaySoundParams};

/// Sound effect identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxId {
    PlaceZone,
    PlaceRoad,
    FireCrackle,
    FireAlarm,
    Demolish,
    UiClick,
    UiOpen,
    UiClose,
    CashRegister,
    MayorSpeak,
    PowerOn,
    WaterFlow,
    Rotate,
    // Ambient (looping)
    AmbientWind,
    AmbientBirds,
    AmbientCity,
}

impl SfxId {
    fn path(self) -> &'static str {
        match self {
            SfxId::PlaceZone => "assets/audio/sfx/place_zone.ogg",
            SfxId::PlaceRoad => "assets/audio/sfx/place_road.ogg",
            SfxId::FireCrackle => "assets/audio/sfx/fire_crackle.ogg",
            SfxId::FireAlarm => "assets/audio/sfx/fire_alarm.ogg",
            SfxId::Demolish => "assets/audio/sfx/demolish.ogg",
            SfxId::UiClick => "assets/audio/sfx/ui_click.ogg",
            SfxId::UiOpen => "assets/audio/sfx/ui_open.ogg",
            SfxId::UiClose => "assets/audio/sfx/ui_close.ogg",
            SfxId::CashRegister => "assets/audio/sfx/cash_register.ogg",
            SfxId::MayorSpeak => "assets/audio/sfx/mayor_speak.ogg",
            SfxId::PowerOn => "assets/audio/sfx/power_on.ogg",
            SfxId::WaterFlow => "assets/audio/sfx/water_flow.ogg",
            SfxId::Rotate => "assets/audio/sfx/rotate.ogg",
            SfxId::AmbientWind => "assets/audio/ambient/wind.ogg",
            SfxId::AmbientBirds => "assets/audio/ambient/birds.ogg",
            SfxId::AmbientCity => "assets/audio/ambient/city.ogg",
        }
    }

    fn is_ambient(self) -> bool {
        matches!(self, SfxId::AmbientWind | SfxId::AmbientBirds | SfxId::AmbientCity)
    }
}

/// Sound effects manager.
pub struct SfxManager {
    sounds: HashMap<SfxId, Sound>,
    sfx_volume: f32,
    ambient_volume: f32,
}

impl SfxManager {
    pub fn new() -> Self {
        Self {
            sounds: HashMap::new(),
            sfx_volume: 0.6,
            ambient_volume: 0.2,
        }
    }

    /// Load all SFX and ambient files. Missing files are silently skipped.
    pub async fn load(&mut self) {
        let ids = [
            SfxId::PlaceZone, SfxId::PlaceRoad, SfxId::FireCrackle,
            SfxId::FireAlarm, SfxId::Demolish, SfxId::UiClick,
            SfxId::UiOpen, SfxId::UiClose, SfxId::CashRegister,
            SfxId::MayorSpeak, SfxId::PowerOn, SfxId::WaterFlow,
            SfxId::Rotate, SfxId::AmbientWind, SfxId::AmbientBirds,
            SfxId::AmbientCity,
        ];

        for id in ids {
            if let Ok(sound) = load_sound(id.path()).await {
                self.sounds.insert(id, sound);
            }
        }

        let count = self.sounds.len();
        if count > 0 {
            eprintln!("[sfx] Loaded {} sounds", count);
        }
    }

    /// Play a one-shot sound effect.
    pub fn play(&self, id: SfxId) {
        if let Some(sound) = self.sounds.get(&id) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: self.sfx_volume,
                },
            );
        }
    }

    /// Play a looping ambient sound at ambient volume.
    pub fn play_ambient(&self, id: SfxId) {
        if let Some(sound) = self.sounds.get(&id) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume: self.ambient_volume,
                },
            );
        }
    }

    /// Start ambient layers based on city population.
    pub fn update_ambient(&self, population: u32) {
        // Wind always plays (empty land feel)
        // Birds play at low pop
        // City hum plays at higher pop
        // These are called once when population crosses thresholds
        // (actual implementation would track which are playing — simplified here)
        let _ = population; // Ambient is started once via play_ambient
    }

    /// Whether any sounds were loaded.
    pub fn has_sounds(&self) -> bool {
        !self.sounds.is_empty()
    }
}
