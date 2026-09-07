//! Developer helpers, all off unless an environment variable enables them.
//!
//! `FLAPPY_SCREENSHOT_DIR=<dir>`: play the game on autopilot for a few
//! seconds, save `title.png`, `playing.png`, and `late.png` into `<dir>`,
//! then quit. Handy for checking the look without a person at the keyboard.

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use flappy_core::*;
use std::path::PathBuf;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(dir) = std::env::var("FLAPPY_SCREENSHOT_DIR") {
            app.insert_resource(ScreenshotRun {
                dir: PathBuf::from(dir),
                shots: vec![(0.6, "title.png"), (3.6, "playing.png"), (6.0, "late.png")],
                elapsed: 0.0,
                started: false,
            })
            .add_systems(Update, (autopilot, take_screenshots).chain());
        }
    }
}

#[derive(Resource)]
struct ScreenshotRun {
    dir: PathBuf,
    /// Remaining `(seconds, file name)` captures, in order.
    shots: Vec<(f32, &'static str)>,
    elapsed: f32,
    started: bool,
}

/// Start the game after a moment, then flap whenever the bird is falling
/// through the lower half of the screen so it stays airborne.
fn autopilot(
    time: Res<Time>,
    mut run: ResMut<ScreenshotRun>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut bird: Single<(&mut Bird, &Transform)>,
    mut flapped: MessageWriter<Flapped>,
) {
    run.elapsed += time.delta_secs();
    let (bird, transform) = &mut *bird;

    match state.get() {
        GameState::Ready if run.elapsed > 1.0 && !run.started => {
            run.started = true;
            bird.velocity = FLAP_VELOCITY;
            flapped.write(Flapped);
            next_state.set(GameState::Playing);
        }
        GameState::Playing if bird.velocity < -150.0 && transform.translation.y < 30.0 => {
            bird.velocity = FLAP_VELOCITY;
            flapped.write(Flapped);
        }
        _ => {}
    }
}

fn take_screenshots(
    mut commands: Commands,
    mut run: ResMut<ScreenshotRun>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(&(at, name)) = run.shots.first()
        && run.elapsed >= at
    {
        run.shots.remove(0);
        let path = run.dir.join(name);
        info!("saving screenshot to {}", path.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }

    // Give the last capture a moment to hit the disk before quitting.
    if run.shots.is_empty() && run.elapsed > 7.0 {
        exit.write(AppExit::Success);
    }
}
