# SlideCity Music — Suno Generation Prompts

Generate these tracks on [suno.com](https://suno.com) and save as OGG files in this directory.
Each track should be 2-4 minutes long, loopable (except Monument sting).

## Track List

### 1. empty_land.ogg — Empty Land
**Mood:** Quiet, anticipatory, hopeful
**Prompt:** Gentle ambient music, soft wind sounds, distant birds, minimal piano notes, peaceful empty landscape, lo-fi, relaxed, city builder game soundtrack, 90 BPM

### 2. first_streets.ogg — First Streets
**Mood:** Optimistic beginning, small-town charm
**Prompt:** Light acoustic guitar, soft percussion, hopeful melody, small town morning vibes, indie folk instrumental, warm and inviting, city builder game, 100 BPM

### 3. growing_city.ogg — Growing City
**Mood:** Energy building, progress, momentum
**Prompt:** Upbeat electronic lo-fi, driving beat, synth arpeggios, feeling of growth and progress, city building montage music, optimistic energy, 120 BPM

### 4. boom_town.ogg — Boom Town
**Mood:** Thriving, prosperous, bustling
**Prompt:** Energetic jazz-electronic fusion, busy city vibes, saxophone melody over electronic beat, prosperous bustling metropolis, SimCity inspired, confident and vibrant, 130 BPM

### 5. disaster.ogg — Disaster
**Mood:** Urgent, tense, alarming
**Prompt:** Tense orchestral strings, urgent percussion, alarm-like synth pulses, disaster emergency music, dramatic and intense, city under threat, 140 BPM

### 6. recovery.ogg — Recovery
**Mood:** Melancholy but hopeful, rebuilding
**Prompt:** Melancholy piano with gentle strings, bittersweet melody, hope after loss, rebuilding theme, emotional but forward-looking, city recovery soundtrack, 85 BPM

### 7. decline.ogg — Decline
**Mood:** Somber, struggling, minor key
**Prompt:** Dark ambient, minor key piano, slow tempo, feeling of urban decay and struggle, somber city at night, lonely and atmospheric, 70 BPM

### 8. monument.ogg — Monument Sting (One-shot, 15-30 seconds)
**Mood:** Triumphant, celebratory, achievement
**Prompt:** Triumphant orchestral fanfare, brass and strings, achievement unlocked moment, grand celebration, heroic accomplishment, city milestone reached, 10 seconds, crescendo to finale

## File Format
- **Format:** OGG Vorbis (preferred by Macroquad)
- **Sample rate:** 44100 Hz
- **Channels:** Stereo
- **Quality:** 6-8 (good quality, reasonable file size)

## Converting from MP3/WAV
If Suno outputs MP3, convert with ffmpeg:
```bash
ffmpeg -i track.mp3 -c:a libvorbis -q:a 6 track.ogg
```

## Notes
- All tracks except Monument should be designed to loop seamlessly
- Keep volume levels consistent across tracks
- The game crossfades between tracks over 3 seconds
- Monument sting plays over the current track (layered, not replacing)
