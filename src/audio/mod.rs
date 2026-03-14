pub mod mood;
pub mod spotify;

use std::collections::HashMap;
use macroquad::audio::{load_sound, play_sound, set_sound_volume, stop_sound, Sound, PlaySoundParams};

use mood::TrackId;

/// Audio backend selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioBackend {
    Local,   // Macroquad audio with OGG files
    Spotify, // Spotify Web API playback control
    Silent,  // No audio (fallback)
}

/// Unified audio manager that handles crossfading and backend selection.
pub struct AudioManager {
    pub backend: AudioBackend,
    // Local audio state
    tracks: HashMap<TrackId, Sound>,
    current_track: Option<TrackId>,
    next_track: Option<TrackId>,
    fade_timer: f32,
    fade_duration: f32,
    current_volume: f32,
    next_volume: f32,
    // Spotify state
    pub spotify: spotify::SpotifyController,
    // Track display
    pub current_mood_label: String,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            backend: AudioBackend::Silent,
            tracks: HashMap::new(),
            current_track: None,
            next_track: None,
            fade_timer: 0.0,
            fade_duration: 3.0,
            current_volume: 1.0,
            next_volume: 0.0,
            spotify: spotify::SpotifyController::new(),
            current_mood_label: "Silent".to_string(),
        }
    }

    /// Load local audio files. Sets backend to Local if any succeed.
    pub async fn load_local_tracks(&mut self) {
        let track_ids = [
            TrackId::EmptyLand,
            TrackId::FirstStreets,
            TrackId::GrowingCity,
            TrackId::BoomTown,
            TrackId::Disaster,
            TrackId::Recovery,
            TrackId::Decline,
            TrackId::Monument,
        ];

        for id in &track_ids {
            let path = format!("assets/audio/{}", id.filename());
            match load_sound(&path).await {
                Ok(sound) => {
                    self.tracks.insert(*id, sound);
                }
                Err(_) => {
                    // Missing audio file — that's fine, we have fallbacks
                }
            }
        }

        if !self.tracks.is_empty() {
            self.backend = AudioBackend::Local;
        }
    }

    /// Transition to a new track (crossfade for local, API call for Spotify).
    pub fn transition_to(&mut self, track_id: TrackId) {
        if self.current_track == Some(track_id) {
            return;
        }
        if self.next_track == Some(track_id) {
            return;
        }

        self.current_mood_label = track_id.label().to_string();

        match self.backend {
            AudioBackend::Local => {
                if let Some(sound) = self.tracks.get(&track_id) {
                    play_sound(sound, PlaySoundParams {
                        looped: track_id.looped(),
                        volume: 0.0,
                    });
                    self.next_track = Some(track_id);
                    self.next_volume = 0.0;
                    self.fade_timer = 0.0;
                }
            }
            AudioBackend::Spotify => {
                self.spotify.play_track(track_id);
                self.current_track = Some(track_id);
                self.next_track = None;
            }
            AudioBackend::Silent => {
                self.current_track = Some(track_id);
            }
        }
    }

    /// Play a one-shot sting over the current music.
    pub fn play_sting(&self, track_id: TrackId) {
        match self.backend {
            AudioBackend::Local => {
                if let Some(sound) = self.tracks.get(&track_id) {
                    play_sound(sound, PlaySoundParams {
                        looped: false,
                        volume: 1.0,
                    });
                }
            }
            AudioBackend::Spotify => {
                // Spotify doesn't support overlaying tracks easily
                // Just log that the sting should play
            }
            AudioBackend::Silent => {}
        }
    }

    /// Update crossfade (call every frame with dt).
    pub fn update(&mut self, dt: f32) {
        if self.backend != AudioBackend::Local {
            return;
        }

        if let Some(next_id) = self.next_track {
            self.fade_timer += dt;
            let t = (self.fade_timer / self.fade_duration).min(1.0);

            // Fade out current
            self.current_volume = 1.0 - t;
            if let Some(curr_id) = self.current_track {
                if let Some(sound) = self.tracks.get(&curr_id) {
                    set_sound_volume(sound, self.current_volume);
                }
            }

            // Fade in next
            self.next_volume = t;
            if let Some(sound) = self.tracks.get(&next_id) {
                set_sound_volume(sound, self.next_volume);
            }

            // Crossfade complete
            if t >= 1.0 {
                if let Some(curr_id) = self.current_track {
                    if let Some(sound) = self.tracks.get(&curr_id) {
                        stop_sound(sound);
                    }
                }
                self.current_track = Some(next_id);
                self.current_volume = 1.0;
                self.next_track = None;
                self.next_volume = 0.0;
                self.fade_timer = 0.0;
            }
        }
    }
}
