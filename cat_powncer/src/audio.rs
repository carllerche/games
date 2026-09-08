//! Synthesized sound effects. No audio files are needed: every effect is a
//! short sequence of sine-wave notes built with Bevy's [`Pitch`] asset.

use crate::Sfx;
use bevy::audio::Volume;
use bevy::prelude::*;
use std::time::Duration;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (queue_sfx, play_notes));
    }
}

/// A note waiting for its start time.
#[derive(Component)]
struct Note {
    delay: Timer,
    freq: f32,
    length: f32,
    volume: f32,
}

fn note(freq: f32, at: f32, length: f32, volume: f32) -> Note {
    Note {
        delay: Timer::from_seconds(at, TimerMode::Once),
        freq,
        length,
        volume,
    }
}

fn queue_sfx(mut reader: MessageReader<Sfx>, mut commands: Commands) {
    for sfx in reader.read() {
        let notes: Vec<Note> = match sfx {
            Sfx::Hop => vec![note(520.0, 0.0, 0.05, 0.25), note(780.0, 0.05, 0.05, 0.2)],
            Sfx::LongHop => vec![
                note(440.0, 0.0, 0.06, 0.25),
                note(660.0, 0.06, 0.06, 0.25),
                note(880.0, 0.12, 0.08, 0.2),
            ],
            Sfx::Bump => vec![note(180.0, 0.0, 0.08, 0.3)],
            Sfx::Swipe => vec![note(900.0, 0.0, 0.04, 0.2), note(1300.0, 0.04, 0.04, 0.15)],
            Sfx::Hit => vec![note(300.0, 0.0, 0.08, 0.3), note(200.0, 0.08, 0.1, 0.3)],
            Sfx::Yelp => vec![note(700.0, 0.0, 0.08, 0.3), note(1000.0, 0.08, 0.12, 0.3)],
            Sfx::DogGone => vec![
                note(900.0, 0.0, 0.08, 0.3),
                note(700.0, 0.08, 0.08, 0.3),
                note(500.0, 0.16, 0.16, 0.3),
            ],
            Sfx::Lick => vec![note(160.0, 0.0, 0.12, 0.35), note(120.0, 0.12, 0.18, 0.35)],
            Sfx::Fall => vec![
                note(600.0, 0.0, 0.1, 0.3),
                note(450.0, 0.1, 0.1, 0.3),
                note(300.0, 0.2, 0.1, 0.3),
                note(150.0, 0.3, 0.2, 0.3),
            ],
            Sfx::Collect => vec![
                note(523.0, 0.0, 0.1, 0.3),
                note(659.0, 0.1, 0.1, 0.3),
                note(784.0, 0.2, 0.1, 0.3),
                note(1047.0, 0.3, 0.25, 0.3),
            ],
            Sfx::Wrong => vec![
                note(330.0, 0.0, 0.2, 0.35),
                note(262.0, 0.2, 0.2, 0.35),
                note(196.0, 0.4, 0.4, 0.35),
            ],
            Sfx::LevelStart => vec![note(659.0, 0.0, 0.1, 0.25), note(988.0, 0.12, 0.2, 0.25)],
            Sfx::Win => vec![
                note(523.0, 0.0, 0.15, 0.3),
                note(659.0, 0.15, 0.15, 0.3),
                note(784.0, 0.3, 0.15, 0.3),
                note(1047.0, 0.45, 0.15, 0.3),
                note(784.0, 0.6, 0.15, 0.3),
                note(1047.0, 0.75, 0.5, 0.3),
            ],
        };
        for n in notes {
            commands.spawn(n);
        }
    }
}

fn play_notes(
    time: Res<Time>,
    mut notes: Query<(Entity, &mut Note)>,
    mut pitches: ResMut<Assets<Pitch>>,
    mut commands: Commands,
) {
    for (entity, mut n) in &mut notes {
        if n.delay.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            commands.spawn((
                AudioPlayer(pitches.add(Pitch::new(n.freq, Duration::from_secs_f32(n.length)))),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(n.volume)),
            ));
        }
    }
}
