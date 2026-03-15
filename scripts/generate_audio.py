#!/usr/bin/env python3
"""
SlideCity Audio Generator
=========================
Synthesizes sound effects and ambient loops using numpy.
No external APIs needed — pure math audio synthesis.

Usage: python3 scripts/generate_audio.py
Output: assets/audio/sfx/*.ogg, assets/audio/ambient/*.ogg
"""

import numpy as np
from pathlib import Path

try:
    import soundfile as sf
except ImportError:
    print("ERROR: pip install soundfile numpy")
    raise SystemExit(1)

ROOT = Path(__file__).parent.parent
SFX_DIR = ROOT / "assets" / "audio" / "sfx"
AMBIENT_DIR = ROOT / "assets" / "audio" / "ambient"
SR = 44100  # Sample rate


def save_ogg(path: Path, data: np.ndarray):
    """Save audio data as OGG Vorbis."""
    path.parent.mkdir(parents=True, exist_ok=True)
    # Normalize to prevent clipping
    peak = np.max(np.abs(data))
    if peak > 0:
        data = data / peak * 0.85
    sf.write(str(path), data.astype(np.float32), SR, format="OGG", subtype="VORBIS")
    print(f"  OK  {path.relative_to(ROOT)}")


def sine(freq, duration, sr=SR):
    n = int(sr * duration)
    t = np.linspace(0, duration, n, endpoint=False)
    return np.sin(2 * np.pi * freq * t)


def noise(duration, sr=SR):
    return np.random.randn(int(sr * duration))


def mix_into(target, source, pos):
    """Safely mix source into target at position, handling length mismatches."""
    end = min(pos + len(source), len(target))
    actual = end - pos
    if actual > 0:
        target[pos:end] += source[:actual]


def envelope(data, attack=0.01, decay=0.0, sustain=1.0, release=0.05):
    """Simple ADSR envelope."""
    n = len(data)
    env = np.ones(n)
    a_samples = int(attack * SR)
    r_samples = int(release * SR)
    d_samples = int(decay * SR)

    # Attack
    if a_samples > 0:
        env[:a_samples] = np.linspace(0, 1, a_samples)
    # Decay
    if d_samples > 0:
        start = a_samples
        end = min(start + d_samples, n)
        env[start:end] = np.linspace(1, sustain, end - start)
    # Sustain
    if d_samples > 0:
        env[a_samples + d_samples:max(0, n - r_samples)] = sustain
    # Release
    if r_samples > 0 and r_samples < n:
        env[-r_samples:] = np.linspace(env[-r_samples] if r_samples < n else sustain, 0, r_samples)
    return data * env


def lowpass(data, cutoff=2000, sr=SR):
    """Simple single-pole lowpass filter."""
    rc = 1.0 / (2 * np.pi * cutoff)
    dt = 1.0 / sr
    alpha = dt / (rc + dt)
    out = np.zeros_like(data)
    out[0] = alpha * data[0]
    for i in range(1, len(data)):
        out[i] = out[i-1] + alpha * (data[i] - out[i-1])
    return out


# ===== SOUND EFFECTS =====

def gen_place_zone():
    """Soft thud when placing a building zone."""
    dur = 0.25
    # Low thud + soft click
    thud = sine(80, dur) * 0.6 + sine(120, dur) * 0.3
    click = noise(0.02) * 0.4
    click = np.pad(click, (0, int(SR * dur) - len(click)))
    data = envelope(thud + click, attack=0.005, release=0.15)
    save_ogg(SFX_DIR / "place_zone.ogg", data)


def gen_place_road():
    """Gravelly road placement sound."""
    dur = 0.3
    gravel = lowpass(noise(dur), 3000) * 0.5
    tap = sine(200, 0.05) * 0.3
    tap = np.pad(tap, (0, int(SR * dur) - len(tap)))
    data = envelope(gravel + tap, attack=0.01, release=0.2)
    save_ogg(SFX_DIR / "place_road.ogg", data)


def gen_fire_crackle():
    """Looping fire crackle (2 seconds)."""
    dur = 2.0
    n = int(SR * dur)
    # Filtered noise bursts
    crackle = np.zeros(n)
    for _ in range(200):
        pos = np.random.randint(0, n - 500)
        burst_len = np.random.randint(100, 500)
        burst = noise(burst_len / SR) * np.random.uniform(0.2, 0.8)
        actual_len = min(burst_len, len(burst), n - pos)
        crackle[pos:pos + actual_len] += burst[:actual_len]
    crackle = lowpass(crackle, 4000)
    # Add some low roar
    roar = lowpass(noise(dur), 800) * 0.3
    data = (crackle * 0.7 + roar) * 0.6
    # Crossfade ends for seamless loop
    fade = int(SR * 0.1)
    data[:fade] *= np.linspace(0, 1, fade)
    data[-fade:] *= np.linspace(1, 0, fade)
    save_ogg(SFX_DIR / "fire_crackle.ogg", data)


def gen_fire_alarm():
    """Short alarm sound when fire starts."""
    dur = 0.6
    t = np.linspace(0, dur, int(SR * dur), endpoint=False)
    # Oscillating siren
    freq = 600 + 200 * np.sin(2 * np.pi * 8 * t)
    alarm = np.sin(2 * np.pi * freq * t / SR * np.cumsum(np.ones_like(t)))
    # Actually, simpler approach
    alarm = np.sin(2 * np.pi * (600 + 200 * np.sin(2 * np.pi * 8 * t)) * t)
    data = envelope(alarm * 0.5, attack=0.02, release=0.1)
    save_ogg(SFX_DIR / "fire_alarm.ogg", data)


def gen_demolish():
    """Crash/rubble destruction sound."""
    dur = 0.5
    crash = lowpass(noise(dur), 2000) * 0.7
    # Low boom
    boom = sine(50, 0.15) * 0.5
    boom = np.pad(boom, (0, int(SR * dur) - len(boom)))
    # Debris rattle
    rattle = lowpass(noise(dur) * sine(15, dur), 3000) * 0.3
    data = envelope(crash + boom + rattle, attack=0.005, release=0.3)
    save_ogg(SFX_DIR / "demolish.ogg", data)


def gen_ui_click():
    """Crisp UI button click."""
    dur = 0.08
    click = sine(800, dur) * 0.3 + sine(1200, dur) * 0.2
    data = envelope(click, attack=0.002, release=0.04)
    save_ogg(SFX_DIR / "ui_click.ogg", data)


def gen_ui_open():
    """Modal/panel open swoosh."""
    dur = 0.2
    t = np.linspace(0, dur, int(SR * dur), endpoint=False)
    # Rising sweep
    freq = 300 + 600 * (t / dur)
    sweep = np.sin(2 * np.pi * freq * t) * 0.3
    soft_noise = lowpass(noise(dur), 5000) * 0.1
    data = envelope(sweep + soft_noise, attack=0.01, release=0.1)
    save_ogg(SFX_DIR / "ui_open.ogg", data)


def gen_ui_close():
    """Modal close sound (falling tone)."""
    dur = 0.15
    t = np.linspace(0, dur, int(SR * dur), endpoint=False)
    freq = 800 - 400 * (t / dur)
    sweep = np.sin(2 * np.pi * freq * t) * 0.25
    data = envelope(sweep, attack=0.005, release=0.08)
    save_ogg(SFX_DIR / "ui_close.ogg", data)


def gen_cash_register():
    """Ka-ching! Money sound."""
    dur = 0.35
    n = int(SR * dur)
    # Bell hit
    bell = sine(2000, dur) * 0.3 + sine(2500, dur) * 0.2
    # Short high ting
    ting = sine(3000, 0.1) * 0.15
    ting = np.pad(ting, (0, n - len(ting)))[:n]
    # Mechanical click
    click = np.pad(noise(0.01) * 0.3, (int(SR * 0.05), 0))[:n]
    click = np.pad(click, (0, max(0, n - len(click))))[:n]
    data = envelope(bell + ting + click, attack=0.002, release=0.25)
    save_ogg(SFX_DIR / "cash_register.ogg", data)


def gen_mayor_speak():
    """Mumble/gibberish when mayor responds (like Animal Crossing)."""
    dur = 0.8
    n = int(SR * dur)
    data = np.zeros(n)
    # Random short vowel-like tones
    pos = 0
    while pos < n - 2000:
        freq = np.random.choice([200, 250, 300, 350, 180, 280])
        syllable_len = np.random.randint(1500, 4000)
        syllable_len = min(syllable_len, n - pos)
        syllable = sine(freq, syllable_len / SR) * 0.4
        # Add formant-like harmonics
        syllable += sine(freq * 2, syllable_len / SR) * 0.15
        syllable += sine(freq * 3, syllable_len / SR) * 0.05
        syllable = envelope(syllable, attack=0.01, release=0.03)
        mix_into(data, syllable, pos)
        pos += syllable_len + np.random.randint(500, 1500)
    data = envelope(data, attack=0.01, release=0.1)
    save_ogg(SFX_DIR / "mayor_speak.ogg", data)


def gen_power_on():
    """Electrical hum when power connects."""
    dur = 0.4
    hum = sine(60, dur) * 0.3 + sine(120, dur) * 0.2
    spark = noise(0.05) * 0.4
    spark = np.pad(spark, (0, int(SR * dur) - len(spark)))
    data = envelope(hum + spark, attack=0.02, release=0.2)
    save_ogg(SFX_DIR / "power_on.ogg", data)


def gen_water_flow():
    """Water rushing/flowing sound."""
    dur = 0.5
    n = int(SR * dur)
    flow = lowpass(noise(dur), 1500) * 0.4
    # Bubble sounds
    bubble = np.zeros(n)
    b1 = sine(400, 0.05) * 0.2
    b2 = sine(500, 0.05) * 0.1
    offset = int(SR * 0.1)
    bubble[offset:offset + len(b1)] += b1
    bubble[offset + 1000:offset + 1000 + len(b2)] += b2
    data = envelope(flow + bubble[:n], attack=0.05, release=0.2)
    save_ogg(SFX_DIR / "water_flow.ogg", data)


def gen_rotate():
    """Whoosh for camera rotation."""
    dur = 0.25
    whoosh = lowpass(noise(dur), 4000) * 0.3
    data = envelope(whoosh, attack=0.02, release=0.1)
    save_ogg(SFX_DIR / "rotate.ogg", data)


# ===== AMBIENT LOOPS =====

def gen_ambient_wind():
    """Gentle wind loop (8 seconds)."""
    dur = 8.0
    n = int(SR * dur)
    wind = lowpass(noise(dur), 800)
    # Slow volume modulation
    t = np.linspace(0, dur, n, endpoint=False)
    mod = 0.3 + 0.2 * np.sin(2 * np.pi * 0.15 * t) + 0.1 * np.sin(2 * np.pi * 0.07 * t)
    data = wind * mod * 0.4
    # Crossfade for seamless loop
    fade = int(SR * 0.5)
    data[:fade] *= np.linspace(0, 1, fade)
    data[-fade:] *= np.linspace(1, 0, fade)
    save_ogg(AMBIENT_DIR / "wind.ogg", data)


def gen_ambient_birds():
    """Bird chirps loop (8 seconds)."""
    dur = 8.0
    n = int(SR * dur)
    data = np.zeros(n)
    # Random chirps
    for _ in range(12):
        pos = np.random.randint(0, n - 8000)
        freq = np.random.uniform(2000, 4000)
        chirp_dur = np.random.uniform(0.05, 0.15)
        chirp_n = int(SR * chirp_dur)
        t = np.linspace(0, chirp_dur, chirp_n, endpoint=False)
        # Frequency modulated chirp
        chirp = np.sin(2 * np.pi * (freq + 500 * np.sin(2 * np.pi * 30 * t)) * t) * 0.15
        chirp = envelope(chirp, attack=0.005, release=0.02)
        mix_into(data, chirp, pos)
    # Crossfade
    fade = int(SR * 0.5)
    data[:fade] *= np.linspace(0, 1, fade)
    data[-fade:] *= np.linspace(1, 0, fade)
    save_ogg(AMBIENT_DIR / "birds.ogg", data)


def gen_ambient_city():
    """City hum/traffic loop (8 seconds)."""
    dur = 8.0
    n = int(SR * dur)
    t = np.linspace(0, dur, n, endpoint=False)
    # Low traffic rumble
    rumble = lowpass(noise(dur), 300) * 0.3
    # Distant horn honks
    data = rumble.copy()
    for _ in range(3):
        pos = np.random.randint(0, n - 10000)
        horn_dur = np.random.uniform(0.3, 0.6)
        horn_n = int(SR * horn_dur)
        freq = np.random.choice([350, 400, 300])
        horn = sine(freq, horn_dur) * 0.08
        horn = envelope(horn, attack=0.02, release=0.1)
        mix_into(data, horn, pos)
    # Subtle engine drone
    drone = sine(80, dur) * 0.1 + sine(160, dur) * 0.05
    mod = 0.5 + 0.5 * np.sin(2 * np.pi * 0.2 * t)
    data += drone * mod
    # Crossfade
    fade = int(SR * 0.5)
    data[:fade] *= np.linspace(0, 1, fade)
    data[-fade:] *= np.linspace(1, 0, fade)
    save_ogg(AMBIENT_DIR / "city.ogg", data)


def main():
    print("SlideCity Audio Generator")
    print("=========================")
    print()

    print("Sound Effects:")
    gen_place_zone()
    gen_place_road()
    gen_fire_crackle()
    gen_fire_alarm()
    gen_demolish()
    gen_ui_click()
    gen_ui_open()
    gen_ui_close()
    gen_cash_register()
    gen_mayor_speak()
    gen_power_on()
    gen_water_flow()
    gen_rotate()
    print()

    print("Ambient Loops:")
    gen_ambient_wind()
    gen_ambient_birds()
    gen_ambient_city()
    print()

    print(f"Done! SFX: {SFX_DIR.relative_to(ROOT)}, Ambient: {AMBIENT_DIR.relative_to(ROOT)}")
    print()
    print("For music tracks, use Suno with prompts from assets/audio/SUNO_PROMPTS.md")


if __name__ == "__main__":
    main()
