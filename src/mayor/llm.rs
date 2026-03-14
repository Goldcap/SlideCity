use std::sync::mpsc;
use std::thread;

use serde_json::{json, Value};

use crate::mayor::personality::MayorPersonality;
use crate::mayor::narration::LogEntry;
use crate::sim::stats::CityStats;

/// Result from the LLM call.
#[derive(Clone, Debug)]
pub enum LlmResult {
    Success(String),
    Error(String),
}

/// Pending LLM request — poll each frame for result.
pub struct LlmRequest {
    receiver: mpsc::Receiver<LlmResult>,
}

impl LlmRequest {
    /// Check if the result is ready (non-blocking).
    pub fn try_recv(&self) -> Option<LlmResult> {
        self.receiver.try_recv().ok()
    }
}

/// Conversation history for multi-turn coherence.
#[derive(Clone, Debug, Default)]
pub struct ConversationHistory {
    pub exchanges: Vec<(String, String)>, // (player, mayor) pairs
}

impl ConversationHistory {
    pub fn push(&mut self, player_msg: String, mayor_response: String) {
        self.exchanges.push((player_msg, mayor_response));
        // Keep last 3 exchanges
        if self.exchanges.len() > 3 {
            self.exchanges.remove(0);
        }
    }
}

/// Check if Claude API is available (API key set).
pub fn api_available() -> bool {
    get_api_key().is_some()
}

/// Send a message to the Claude API in a background thread.
/// Returns a `LlmRequest` that can be polled for the result.
pub fn send_audience_request(
    player_message: String,
    personality: &MayorPersonality,
    stats: &CityStats,
    recent_log: Vec<LogEntry>,
    history: &ConversationHistory,
    funds: i64,
    year: u32,
) -> Option<LlmRequest> {
    let api_key = get_api_key()?;

    let system_prompt = build_system_prompt(personality, stats, &recent_log, funds, year);
    let messages = build_messages(&player_message, history);

    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = call_claude_api(&api_key, &system_prompt, &messages);
        let _ = sender.send(result);
    });

    Some(LlmRequest { receiver })
}

fn get_api_key() -> Option<String> {
    // Try environment variable first
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }

    // Try .env file
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("ANTHROPIC_API_KEY=") {
                let val = val.trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() && !val.starts_with("your_") {
                    return Some(val.to_string());
                }
            }
        }
    }

    None
}

fn build_system_prompt(
    personality: &MayorPersonality,
    stats: &CityStats,
    recent_log: &[LogEntry],
    funds: i64,
    year: u32,
) -> String {
    let mut prompt = format!(
        "You are {}, the mayor of SlideCity — an autonomous city simulation game.\n\n\
         PERSONALITY:\n\
         - Name: {}\n\
         - Emoji: {}\n\
         - Growth aggression: {:.1}/1.0\n\
         - Green affinity: {:.1}/1.0\n\
         - Industrial bias: {:.1}/1.0\n\n\
         CURRENT CITY STATE:\n\
         - Population: {}\n\
         - Year: {}\n\
         - Funds: ${}\n\
         - Happiness: {:.0}%\n\
         - Zones: {} residential, {} commercial, {} industrial\n\
         - Power coverage: {:.0}%\n\
         - Water coverage: {:.0}%\n\
         - Active fires: {}\n\n",
        personality.name, personality.name, personality.emoji,
        personality.growth_aggression,
        personality.green_affinity,
        personality.industrial_bias,
        stats.population, year, funds,
        stats.happiness * 100.0,
        stats.res_count, stats.com_count, stats.ind_count,
        stats.power_coverage * 100.0,
        stats.water_coverage * 100.0,
        stats.fire_count,
    );

    // Recent log entries for context
    if !recent_log.is_empty() {
        prompt.push_str("RECENT MAYOR LOG (your recent thoughts):\n");
        for entry in recent_log.iter().take(5) {
            prompt.push_str(&format!("- Year {}, {}: {}\n", entry.year, entry.season, entry.text));
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "INSTRUCTIONS:\n\
         - Respond in character as this mayor personality.\n\
         - Keep responses to 1-3 sentences.\n\
         - You may agree, argue, deflect, or express your personality.\n\
         - Reference the current city state when relevant.\n\
         - Stay in character — you ARE this mayor, not an AI.\n\
         - Be concise and conversational, like a real mayor talking to a citizen.\n"
    );

    prompt
}

fn build_messages(
    player_message: &str,
    history: &ConversationHistory,
) -> Vec<Value> {
    let mut messages = Vec::new();

    // Previous exchanges for continuity
    for (player, mayor) in &history.exchanges {
        messages.push(json!({
            "role": "user",
            "content": player
        }));
        messages.push(json!({
            "role": "assistant",
            "content": mayor
        }));
    }

    // Current message
    messages.push(json!({
        "role": "user",
        "content": player_message
    }));

    messages
}

fn call_claude_api(
    api_key: &str,
    system_prompt: &str,
    messages: &[Value],
) -> LlmResult {
    let body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 256,
        "system": system_prompt,
        "messages": messages,
    });

    let result = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send_json(&body);

    match result {
        Ok(response) => {
            match response.into_json::<Value>() {
                Ok(json) => {
                    if let Some(content) = json["content"].as_array() {
                        if let Some(text) = content.first().and_then(|c| c["text"].as_str()) {
                            // Truncate to 200 chars if needed
                            let truncated = if text.len() > 200 {
                                format!("{}...", &text[..197])
                            } else {
                                text.to_string()
                            };
                            return LlmResult::Success(truncated);
                        }
                    }

                    // Check for error in response
                    if let Some(err) = json["error"]["message"].as_str() {
                        return LlmResult::Error(format!("API error: {}", err));
                    }

                    LlmResult::Error("Unexpected response format".to_string())
                }
                Err(e) => LlmResult::Error(format!("Parse error: {}", e)),
            }
        }
        Err(ureq::Error::Status(429, _)) => {
            LlmResult::Error("The mayor needs rest. (Rate limited)".to_string())
        }
        Err(ureq::Error::Status(401, _)) => {
            LlmResult::Error("Communication breakdown. (Invalid API key)".to_string())
        }
        Err(ureq::Error::Status(code, _)) => {
            LlmResult::Error(format!("The mayor is unavailable. (HTTP {})", code))
        }
        Err(ureq::Error::Transport(e)) => {
            if e.kind() == ureq::ErrorKind::Io {
                LlmResult::Error("The mayor is busy. (Timeout)".to_string())
            } else {
                LlmResult::Error(format!("Communication error: {}", e))
            }
        }
    }
}
