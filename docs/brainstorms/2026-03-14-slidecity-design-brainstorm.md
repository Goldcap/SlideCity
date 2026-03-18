# SlideCity Design Brainstorm
**Date:** 2026-03-14
**Status:** Active
**Participants:** User + Claude

---

## What We're Building

An autonomous isometric city simulation (SimCity 2000 meets Conway's Game of Life) where a Virtual Mayor AI builds and manages the city. The player is primarily a spectator but can **influence the mayor** through a tiered political interaction system.

Built in Rust with Macroquad 0.4, native-first (WASM later).

---

## Key Decisions

### 1. Mayor Personality: Hard Divergence

**Decision:** Mayor personalities cause dramatically different city outcomes. The Machine builds grey industrial sprawl; The Environmentalist creates garden cities. Cities look and feel completely different depending on the mayor.

**Implication:** Some mayors will make objectively bad choices. Cities can fail, decline, and death-spiral. This is intentional — it creates narrative tension and makes the influence system meaningful.

### 2. Player Influence System (New Feature)

**Decision:** Players have a limited resource — **Influence Points (IP)** — spent across three tiers:

| Tier | Cost | Mechanic |
|------|------|----------|
| **Suggestion Box** | Low | Suggest an action. Mayor weighs against personality — might comply, ignore, or argue back in the log. |
| **Council Vote** | Medium | Mayor proposes 2-3 actions, player picks one. Mayor may still grumble or resist. |
| **Direct Audience** | High | Free-text chat with the mayor, powered by Claude API. Mayor responds in character. High compliance but personality pushback. |

**IP Economy:**
- **Passive:** +1 IP per game year
- **Milestones:** +1 IP for population thresholds, phase transitions, surviving disasters
- **Treasury:** Can buy IP with city funds (budget drain)
- **Chaos:** Triggering disasters earns IP (moral hazard by design!)

The disaster-for-IP loop is a deliberate feedback mechanism: players are incentivized to create chaos to earn face time with the mayor. Burn the east side, then tell The Machine "maybe build a park this time."

### 3. LLM-Powered Mayor Conversations

**Decision:** The Direct Audience tier uses the Claude API. Each mayor archetype becomes a system prompt incorporating:
- Personality traits and biases
- Current city state (population, happiness, funds, active crises)
- Recent history (what they've built, what's failed)
- The player's suggestion

The mayor responds in character — arguing, deflecting, or reluctantly agreeing based on personality.

### 4. Mayor Selection Screen

**Decision:** Civ 5-style selection at game start:
- All 8 mayors always available
- Show portrait + name + emoji, but **no personality stats revealed**
- Player discovers the mayor's tendencies through gameplay
- Each archetype has distinct **powers and weaknesses** (mechanical bonuses/penalties):
  - Example: The Developer — roads cost 50% less, but parks decay 2x faster
  - Example: The Environmentalist — parks prevent fire spread, but industrial tax income halved
  - Example: The Machine — industrial output doubled, but happiness decays 2x faster
- Personality weights randomize slightly each game for variety

### 5. Difficulty Selection

**Decision:** Player chooses difficulty at game start. Difficulty affects:
- Fire spread probability
- Decay/abandonment rates
- Fund pressure (starting funds, tax rates, placement costs)
- Disaster frequency

### 6. Simulation Pacing: Tunable Tick Speed

**Decision:** Ship with spec values as the baseline, expose a speed slider (1x / 2x / 4x / 8x). Player controls the pacing. Debug mode gets additional step-by-step controls.

### 7. Visual Presentation: Juicy From the Start

**Decision:** Don't wait for polish — build visual juice into the renderer from day one:
- Day/night cycle tinting
- Particle effects (smoke from industrial, sparks from fire)
- Building pop-in animations
- Screen shake on disasters
- Smooth camera pans on mayor actions

### 8. Art Pipeline: Open to Alternatives

**Decision:** Retro-diffusion via Replicate is the first choice for sprites, but untested. Plan for:
1. Try retro-diffusion first — evaluate consistency across the ~40 sprite set
2. If inconsistent: try other AI models or curated asset packs
3. Colored-rect fallbacks must always work perfectly — this is the baseline experience

### 9. Dynamic Music: Core Feature

**Decision:** The crossfading contextual music system is essential, not a polish pass. The city's emotional arc is told through sound:
- Empty land ambient at pop 0
- Hopeful acoustic at first streets
- Urban groove during growth
- Triumphant energy at boom
- Tense disaster tracks
- Monument victory sting

Build the audio system early, even with placeholder tracks. Suno for generation, but open to alternatives.

### 10. Build Target: Native First

**Decision:** Target native (macOS/Linux/Windows) first. WASM port is nice-to-have for later — avoids browser audio autoplay restrictions and filesystem constraints during core development.

---

## Open Questions

1. **Mayor powers/weaknesses balance** — What are the specific mechanical bonuses for all 8 archetypes? Need to design these so every mayor is interesting but not all equally optimal. (Defer to planning/playtesting phase.)

2. **IP costs and earning rates** — What are the exact costs for each tier? How many IPs should a typical game generate? (Defer to playtesting — start with rough values and tune.)

3. **Save/load with influence state** — The spec mentions serde for save/load. Need to persist IP balance, mayor conversation history, and difficulty settings.

## Resolved Questions

- **Claude API fallback:** Ship with a lightweight proxy that rate-limits free Claude API calls. Everyone gets a taste of LLM mayor chat; power users bring their own key.

- **Start screen:** Rich presentation — mayor selection with animated portraits, difficulty picker, map seed input, game speed default. Sets the mood before the city loads.

---

## Approach: Why This Design

The original spec describes a pure spectator experience. This brainstorm adds a **political influence layer** that transforms the player from passive viewer to reluctant participant — you *can* just watch, but you also have the tools to nudge, argue with, and bribe your mayor.

The genius of the IP-from-disasters mechanic is that it creates a moral dilemma: the easiest way to influence your city's future is to burn it down first. That's emergent storytelling, not scripted narrative.

The hard-divergence personality system means every run tells a different story, and the LLM-powered conversations make each mayor feel genuinely alive.

---

## Next Steps

Run `/ce:plan` to transform this into an implementation plan.
