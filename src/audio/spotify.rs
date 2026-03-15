use std::collections::HashMap;

use super::mood::TrackId;

/// Default Spotify track URIs mapped to each mood.
/// Users can override these in a config file.
const DEFAULT_TRACK_MAP: &[(TrackId, &str)] = &[
    // These are placeholder URIs — replace with actual curated tracks
    (TrackId::EmptyLand, "spotify:track:placeholder_empty_land"),
    (TrackId::FirstStreets, "spotify:track:placeholder_first_streets"),
    (TrackId::GrowingCity, "spotify:track:placeholder_growing_city"),
    (TrackId::BoomTown, "spotify:track:placeholder_boom_town"),
    (TrackId::Disaster, "spotify:track:placeholder_disaster"),
    (TrackId::Recovery, "spotify:track:placeholder_recovery"),
    (TrackId::Decline, "spotify:track:placeholder_decline"),
    (TrackId::Monument, "spotify:track:placeholder_monument"),
];

/// Spotify Web API playback controller.
/// Uses PKCE auth flow for native desktop apps.
pub struct SpotifyController {
    /// OAuth access token (obtained via PKCE flow).
    access_token: Option<String>,
    /// Refresh token for token renewal.
    refresh_token: Option<String>,
    /// Spotify Client ID (from .env or config).
    client_id: Option<String>,
    /// Track URI mapping: TrackId → Spotify URI.
    track_map: HashMap<TrackId, String>,
    /// Currently playing track.
    current_track: Option<TrackId>,
    /// Whether Spotify integration is available.
    pub available: bool,
    /// Status message for UI display.
    pub status: String,
}

impl SpotifyController {
    pub fn new() -> Self {
        let mut track_map = HashMap::new();
        for &(id, uri) in DEFAULT_TRACK_MAP {
            track_map.insert(id, uri.to_string());
        }

        // Check for Spotify client ID in environment
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").ok();
        let available = client_id.is_some();

        Self {
            access_token: None,
            refresh_token: None,
            client_id,
            track_map,
            current_track: None,
            available,
            status: if available {
                "Spotify: ready to authenticate".to_string()
            } else {
                "Spotify: no SPOTIFY_CLIENT_ID set".to_string()
            },
        }
    }

    /// Check if Spotify is authenticated and ready.
    pub fn is_authenticated(&self) -> bool {
        self.access_token.is_some()
    }

    /// Start the OAuth PKCE authentication flow.
    /// Opens the user's browser to Spotify's auth page.
    /// Returns the auth URL to open.
    pub fn start_auth_flow(&self) -> Option<String> {
        let client_id = self.client_id.as_ref()?;

        // Generate PKCE code verifier and challenge
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        let scopes = "user-modify-playback-state user-read-playback-state user-read-currently-playing";
        let redirect_uri = "http://127.0.0.1:8421/callback";

        let auth_url = format!(
            "https://accounts.spotify.com/authorize?\
            client_id={}&\
            response_type=code&\
            redirect_uri={}&\
            scope={}&\
            code_challenge_method=S256&\
            code_challenge={}",
            client_id,
            urlencoded(redirect_uri),
            urlencoded(scopes),
            code_challenge,
        );

        // TODO: Store code_verifier for the token exchange step
        // TODO: Start a local HTTP server on 127.0.0.1:8421 to catch the callback
        // TODO: Exchange auth code for access token using PKCE

        Some(auth_url)
    }

    /// Set custom track mapping (e.g., from config file).
    pub fn set_track_uri(&mut self, track_id: TrackId, uri: String) {
        self.track_map.insert(track_id, uri);
    }

    /// Play a track on the user's Spotify device.
    pub fn play_track(&mut self, track_id: TrackId) {
        if !self.is_authenticated() {
            return;
        }
        if self.current_track == Some(track_id) {
            return;
        }

        if let Some(uri) = self.track_map.get(&track_id) {
            self.current_track = Some(track_id);
            self.status = format!("Playing: {}", track_id.label());

            // TODO: Async HTTP call to Spotify Web API
            // PUT https://api.spotify.com/v1/me/player/play
            // Body: { "uris": ["spotify:track:xxx"] }
            // Header: Authorization: Bearer {access_token}
            let _uri = uri.clone();
            let _token = self.access_token.clone();

            // For now, log the intent. Full async implementation needs
            // a background task runner (tokio or macroquad's coroutines).
        }
    }

    /// Pause playback.
    pub fn pause(&self) {
        if !self.is_authenticated() {
            return;
        }
        // TODO: PUT https://api.spotify.com/v1/me/player/pause
    }

    /// Set access token after completing OAuth flow.
    pub fn set_tokens(&mut self, access_token: String, refresh_token: Option<String>) {
        self.access_token = Some(access_token);
        self.refresh_token = refresh_token;
        self.available = true;
        self.status = "Spotify: connected".to_string();
    }
}

// ===== PKCE Helpers =====

fn generate_code_verifier() -> String {
    use ::rand::Rng;
    let mut rng = ::rand::thread_rng();
    let bytes: Vec<u8> = (0..64).map(|_| rng.gen_range(b'a'..=b'z')).collect();
    String::from_utf8(bytes).unwrap()
}

fn generate_code_challenge(verifier: &str) -> String {
    // SHA-256 hash, then base64url encode
    // For now, return the verifier as a placeholder.
    // Full implementation needs sha2 crate.
    // TODO: Add sha2 dependency and implement proper S256 challenge
    let _ = verifier;
    "placeholder_challenge".to_string()
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
}
