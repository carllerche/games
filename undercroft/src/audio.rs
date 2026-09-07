//! Sound effects and music synthesized at startup. Everything is rendered to
//! 16-bit mono WAV in memory, so the game ships no audio files.

use bevy::{audio::Volume, platform::collections::HashMap, prelude::*};
use std::f32::consts::TAU;

const RATE: u32 = 22050;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxKind {
    Swing,
    Hit,
    Hurt,
    Coin,
    Potion,
    Bow,
    ArrowHit,
    EnemyDie,
    Stairs,
    Buy,
    LevelUp,
    Error,
    Break,
    Chest,
    Key,
    Roar,
    Menu,
    Select,
    Teleport,
    Spike,
    Perk,
    Death,
    Victory,
}

impl SfxKind {
    const ALL: [SfxKind; 23] = [
        Self::Swing,
        Self::Hit,
        Self::Hurt,
        Self::Coin,
        Self::Potion,
        Self::Bow,
        Self::ArrowHit,
        Self::EnemyDie,
        Self::Stairs,
        Self::Buy,
        Self::LevelUp,
        Self::Error,
        Self::Break,
        Self::Chest,
        Self::Key,
        Self::Roar,
        Self::Menu,
        Self::Select,
        Self::Teleport,
        Self::Spike,
        Self::Perk,
        Self::Death,
        Self::Victory,
    ];
}

#[derive(Message)]
pub struct PlaySfx(pub SfxKind);

#[derive(Resource)]
struct SoundBank {
    sfx: HashMap<SfxKind, Handle<AudioSource>>,
    music: Handle<AudioSource>,
}

#[derive(Component)]
struct Music;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfx>()
            .add_systems(Startup, build_bank)
            .add_systems(Update, play_sfx);
    }
}

fn build_bank(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let mut sfx = HashMap::new();
    for kind in SfxKind::ALL {
        let samples = synth_sfx(kind);
        sfx.insert(kind, sources.add(AudioSource { bytes: wav(&samples).into() }));
    }
    let music = sources.add(AudioSource { bytes: wav(&synth_music()).into() });
    commands.spawn((
        Music,
        AudioPlayer::new(music.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.35)),
    ));
    commands.insert_resource(SoundBank { sfx, music });
}

fn play_sfx(mut commands: Commands, mut messages: MessageReader<PlaySfx>, bank: Option<Res<SoundBank>>) {
    let Some(bank) = bank else { return };
    let _ = &bank.music;
    for PlaySfx(kind) in messages.read() {
        let volume = match kind {
            SfxKind::Swing | SfxKind::Menu => 0.35,
            SfxKind::Coin => 0.4,
            SfxKind::Roar | SfxKind::Death | SfxKind::Victory => 0.8,
            _ => 0.55,
        };
        commands.spawn((
            AudioPlayer::new(bank.sfx[kind].clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
        ));
    }
}

// ---------------------------------------------------------------------------
// Synthesis
// ---------------------------------------------------------------------------

fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn square(phase: f32, duty: f32) -> f32 {
    if phase.fract() < duty { 1.0 } else { -1.0 }
}

fn triangle(phase: f32) -> f32 {
    let p = phase.fract();
    if p < 0.5 { p * 4.0 - 1.0 } else { 3.0 - p * 4.0 }
}

/// Renders `seconds` of a tone whose pitch and amplitude are functions of
/// time (0..1 over the sound), using the given oscillator.
fn tone(seconds: f32, osc: impl Fn(f32, f32) -> f32, freq: impl Fn(f32) -> f32, amp: impl Fn(f32) -> f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut phase = 0.0f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / n as f32;
        phase += freq(t) / RATE as f32;
        out.push(osc(phase, t) * amp(t));
    }
    out
}

fn noise_burst(seconds: f32, amp: impl Fn(f32) -> f32, lowpass: f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut rng = Noise(0x1234_5678);
    let mut out = Vec::with_capacity(n);
    let mut last = 0.0;
    for i in 0..n {
        let t = i as f32 / n as f32;
        last += (rng.next() - last) * lowpass;
        out.push(last * amp(t));
    }
    out
}

fn mix(parts: &[Vec<f32>]) -> Vec<f32> {
    let len = parts.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = vec![0.0; len];
    for part in parts {
        for (o, s) in out.iter_mut().zip(part) {
            *o += s;
        }
    }
    out
}

fn concat(parts: &[Vec<f32>]) -> Vec<f32> {
    parts.iter().flatten().copied().collect()
}

fn decay(t: f32) -> f32 {
    (1.0 - t).powi(2)
}

fn synth_sfx(kind: SfxKind) -> Vec<f32> {
    let sq = |d: f32| move |p: f32, _t: f32| square(p, d);
    let tri = |p: f32, _t: f32| triangle(p);
    match kind {
        SfxKind::Swing => noise_burst(0.12, |t| (1.0 - t) * (t * 8.0).min(1.0) * 0.6, 0.35),
        SfxKind::Hit => mix(&[
            tone(0.09, sq(0.5), |t| 440.0 - t * 300.0, |t| decay(t) * 0.5),
            noise_burst(0.07, |t| decay(t) * 0.5, 0.6),
        ]),
        SfxKind::Hurt => tone(0.25, sq(0.3), |t| 220.0 - t * 120.0 + (t * 60.0).sin() * 20.0, |t| decay(t) * 0.6),
        SfxKind::Coin => concat(&[
            tone(0.06, sq(0.5), |_| 1046.0, |_| 0.4),
            tone(0.14, sq(0.5), |_| 1568.0, |t| decay(t) * 0.4),
        ]),
        SfxKind::Potion => concat(&[
            tone(0.07, tri, |_| 392.0, |_| 0.6),
            tone(0.07, tri, |_| 523.0, |_| 0.6),
            tone(0.07, tri, |_| 659.0, |_| 0.6),
            tone(0.2, tri, |_| 784.0, |t| decay(t) * 0.6),
        ]),
        SfxKind::Bow => mix(&[
            tone(0.15, tri, |t| 180.0 + t * 400.0, |t| decay(t) * 0.5),
            noise_burst(0.08, |t| decay(t) * 0.3, 0.5),
        ]),
        SfxKind::ArrowHit => noise_burst(0.06, |t| decay(t) * 0.5, 0.8),
        SfxKind::EnemyDie => mix(&[
            tone(0.3, sq(0.25), |t| 300.0 - t * 250.0, |t| decay(t) * 0.4),
            noise_burst(0.25, |t| decay(t) * 0.5, 0.3),
        ]),
        SfxKind::Stairs => concat(&[
            tone(0.1, sq(0.5), |_| 659.0, |_| 0.4),
            tone(0.1, sq(0.5), |_| 523.0, |_| 0.4),
            tone(0.1, sq(0.5), |_| 440.0, |_| 0.4),
            tone(0.3, sq(0.5), |_| 330.0, |t| decay(t) * 0.4),
        ]),
        SfxKind::Buy => concat(&[
            tone(0.08, sq(0.5), |_| 784.0, |_| 0.4),
            tone(0.08, sq(0.5), |_| 1046.0, |_| 0.4),
            tone(0.2, sq(0.5), |_| 1318.0, |t| decay(t) * 0.4),
        ]),
        SfxKind::LevelUp => concat(&[
            tone(0.09, sq(0.5), |_| 523.0, |_| 0.4),
            tone(0.09, sq(0.5), |_| 659.0, |_| 0.4),
            tone(0.09, sq(0.5), |_| 784.0, |_| 0.4),
            tone(0.09, sq(0.5), |_| 1046.0, |_| 0.4),
            tone(0.35, sq(0.5), |_| 1318.0, |t| decay(t) * 0.4),
        ]),
        SfxKind::Error => tone(0.2, sq(0.5), |_| 110.0, |t| (1.0 - t) * 0.4),
        SfxKind::Break => mix(&[
            noise_burst(0.35, |t| decay(t) * 0.7, 0.2),
            tone(0.2, sq(0.5), |t| 120.0 - t * 60.0, |t| decay(t) * 0.4),
        ]),
        SfxKind::Chest => concat(&[
            tone(0.08, sq(0.5), |_| 392.0, |_| 0.4),
            tone(0.08, sq(0.5), |_| 494.0, |_| 0.4),
            tone(0.08, sq(0.5), |_| 587.0, |_| 0.4),
            tone(0.25, sq(0.5), |_| 784.0, |t| decay(t) * 0.4),
        ]),
        SfxKind::Key => concat(&[
            tone(0.1, tri, |_| 880.0, |_| 0.5),
            tone(0.1, tri, |_| 1174.0, |_| 0.5),
            tone(0.3, tri, |_| 1760.0, |t| decay(t) * 0.5),
        ]),
        SfxKind::Roar => mix(&[
            tone(0.7, sq(0.5), |t| 80.0 + (t * 40.0).sin() * 15.0 - t * 30.0, |t| (t * 10.0).min(1.0) * (1.0 - t) * 0.6),
            noise_burst(0.7, |t| (1.0 - t) * 0.4, 0.15),
        ]),
        SfxKind::Menu => tone(0.05, sq(0.5), |_| 880.0, |t| decay(t) * 0.3),
        SfxKind::Select => concat(&[
            tone(0.06, sq(0.5), |_| 659.0, |_| 0.35),
            tone(0.12, sq(0.5), |_| 988.0, |t| decay(t) * 0.35),
        ]),
        SfxKind::Teleport => tone(0.35, sq(0.25), |t| 200.0 + t * 1400.0, |t| (1.0 - t) * 0.4),
        SfxKind::Spike => mix(&[
            tone(0.12, sq(0.5), |t| 900.0 - t * 500.0, |t| decay(t) * 0.4),
            noise_burst(0.1, |t| decay(t) * 0.4, 0.7),
        ]),
        SfxKind::Perk => concat(&[
            tone(0.1, tri, |_| 523.0, |_| 0.5),
            tone(0.1, tri, |_| 784.0, |_| 0.5),
            tone(0.1, tri, |_| 659.0, |_| 0.5),
            tone(0.4, tri, |_| 1046.0, |t| decay(t) * 0.5),
        ]),
        SfxKind::Death => concat(&[
            tone(0.2, sq(0.5), |_| 330.0, |_| 0.5),
            tone(0.2, sq(0.5), |_| 311.0, |_| 0.5),
            tone(0.2, sq(0.5), |_| 294.0, |_| 0.5),
            tone(0.8, sq(0.5), |t| 262.0 - t * 100.0, |t| (1.0 - t) * 0.5),
        ]),
        SfxKind::Victory => concat(&[
            tone(0.15, sq(0.5), |_| 523.0, |_| 0.5),
            tone(0.15, sq(0.5), |_| 659.0, |_| 0.5),
            tone(0.15, sq(0.5), |_| 784.0, |_| 0.5),
            tone(0.15, sq(0.5), |_| 1046.0, |_| 0.5),
            tone(0.15, sq(0.5), |_| 784.0, |_| 0.5),
            tone(0.6, sq(0.5), |_| 1046.0, |t| (1.0 - t) * 0.5),
        ]),
    }
}

/// Frequency of a MIDI note number.
fn midi(note: i32) -> f32 {
    440.0 * 2f32.powf((note - 69) as f32 / 12.0)
}

/// A short looping chiptune in A minor: square lead, triangle bass, noise
/// hats and a thumping kick.
fn synth_music() -> Vec<f32> {
    let bpm = 132.0;
    let beat = 60.0 / bpm;
    let eighth = beat / 2.0;

    // Chord roots per bar (MIDI): Am, F, C, G, Am, F, E, E
    let roots = [57, 53, 48, 55, 57, 53, 52, 52];
    // Arpeggio shapes in semitones above the root.
    let shapes: [[i32; 4]; 8] = [
        [0, 3, 7, 12],
        [0, 4, 7, 12],
        [0, 4, 7, 12],
        [0, 4, 7, 12],
        [0, 3, 7, 12],
        [0, 4, 7, 12],
        [0, 4, 7, 11],
        [0, 4, 7, 12],
    ];
    // A melody over the second half so the loop breathes.
    let melody: [i32; 32] = [
        69, 0, 72, 0, 76, 0, 72, 0, 74, 0, 72, 0, 69, 0, 67, 0, //
        69, 72, 76, 79, 76, 72, 74, 0, 71, 0, 68, 0, 64, 0, 0, 0,
    ];

    let mut lead = Vec::new();
    let mut bass = Vec::new();
    let mut drums = Vec::new();
    for pass in 0..2 {
        for (bar, root) in roots.iter().enumerate() {
            for step in 0..8 {
                let shape = shapes[bar];
                let note = root + 12 + shape[step % 4] + if step >= 4 { 12 } else { 0 };
                let note = if pass == 1 {
                    let m = melody[(bar % 4) * 8 + step];
                    if m == 0 { -1 } else { m }
                } else {
                    note
                };
                let seg = if note < 0 {
                    vec![0.0; (eighth * RATE as f32) as usize]
                } else {
                    let f = midi(note);
                    tone(eighth, |p, _| square(p, 0.25), move |_| f, |t| (1.0 - t * 0.6) * 0.16)
                };
                lead.extend(seg);

                let bf = midi(root - 12 + if step % 4 == 3 { 12 } else { 0 });
                bass.extend(tone(eighth, |p, _| triangle(p), move |_| bf, |t| (1.0 - t * 0.3) * 0.28));

                let kick = step % 4 == 0;
                let hat = step % 2 == 1;
                let mut d = vec![0.0; (eighth * RATE as f32) as usize];
                if kick {
                    let k = tone(0.1, |p, _| (p * TAU).sin(), |t| 120.0 - t * 80.0, |t| decay(t) * 0.5);
                    for (o, s) in d.iter_mut().zip(k) {
                        *o += s;
                    }
                }
                if hat {
                    let h = noise_burst(0.04, |t| decay(t) * 0.12, 0.9);
                    for (o, s) in d.iter_mut().zip(h) {
                        *o += s;
                    }
                }
                drums.extend(d);
            }
        }
    }
    mix(&[lead, bass, drums])
}
