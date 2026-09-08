//! Cat Powncer: a cartoon grid-hop platformer for young kids.
//!
//! A cat pounces across a giant floating cat-tree world, swiping toys off the
//! platforms, scratching at slobbery dogs, and hunting for the purple yarn ball
//! that finishes each of the 22 levels.

#![allow(clippy::type_complexity)]

mod audio;
mod cat;
mod dog;
mod grid;
mod level;
mod rng;
mod ui;
mod world;

use bevy::prelude::*;
use bevy::window::PresentMode;
use level::{LEVEL_COUNT, YarnColor};

/// Top-level screens.
#[derive(States, Default, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Title,
    Playing,
    Victory,
}

/// What is happening inside a level. Input is only accepted during `Play`.
#[derive(Resource, Clone, Debug, Default)]
pub enum Phase {
    #[default]
    Play,
    Intro(Timer),
    Wrong {
        timer: Timer,
        color: YarnColor,
    },
    Complete(Timer),
}

impl Phase {
    pub fn is_play(&self) -> bool {
        matches!(self, Phase::Play)
    }
}

pub fn in_play(phase: Res<Phase>) -> bool {
    phase.is_play()
}

/// The level currently being played (1-based).
#[derive(Resource, Clone, Copy, Debug)]
pub struct CurrentLevel(pub u32);

/// A level requested on the command line with `--level N`; used once, then cleared.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct StartLevel(pub Option<u32>);

/// Ask the world to (re)build the level named by [`CurrentLevel`].
#[derive(Message, Default)]
pub struct LoadLevel;

/// Sound effects. Rendered as little synthesized bleeps by the audio module.
#[derive(Message, Clone, Copy, Debug)]
pub enum Sfx {
    Hop,
    LongHop,
    Bump,
    Swipe,
    Hit,
    Yelp,
    DogGone,
    Lick,
    Fall,
    Collect,
    Wrong,
    LevelStart,
    Win,
}

/// Everything spawned for the current level; despawned when the level reloads.
#[derive(Component)]
pub struct LevelEntity;

/// System ordering inside the `Update` schedule while playing.
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GameSet {
    /// Read keyboard / mouse and decide what the cat does.
    Input,
    /// Dogs decide what they do.
    Think,
    /// Advance hops, swipes, licks and other animations.
    Animate,
    /// React to landings, hits, and pickups.
    Resolve,
}

fn main() {
    let start_level = std::env::args()
        .skip_while(|a| a != "--level")
        .nth(1)
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| (1..=LEVEL_COUNT).contains(n));

    if let Some(n) = std::env::args()
        .skip_while(|a| a != "--map")
        .nth(1)
        .and_then(|n| n.parse::<u32>().ok())
    {
        let data = level::generate(n);
        println!("Level {n}: {} islands, {} dogs, {} toys, {} yarn balls", data.islands.len(), data.dogs.len(), data.toys.len(), data.yarn.len());
        print!("{}", level::ascii_map(&data));
        return;
    }

    let mut app = App::new();
    if let Some(req) = screenshot_request() {
        app.insert_resource(req).add_systems(Update, take_screenshot);
    }
    let script: Vec<char> = std::env::args()
        .skip_while(|a| a != "--script")
        .nth(1)
        .map(|s| s.chars().filter(|c| !c.is_whitespace() && *c != ',').collect())
        .unwrap_or_default();
    if !script.is_empty() {
        app.insert_resource(ScriptedInput {
            steps: script,
            timer: Timer::from_seconds(0.5, TimerMode::Once),
        })
        .add_systems(PreUpdate, scripted_input.after(bevy::input::InputSystems))
        .add_systems(PostUpdate, release_scripted_keys);
    }
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Cat Powncer".into(),
                resolution: (1280, 800).into(),
                present_mode: PresentMode::AutoVsync,
                // In the browser, fill the `#game` canvas's parent (the whole
                // page; see `web/`). The camera is orthographic with a fixed
                // vertical extent, so any aspect ratio works.
                canvas: Some("#game".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb_u8(150, 205, 245)))
        .insert_state(if start_level.is_some() { AppState::Playing } else { AppState::Title })
        .init_resource::<Phase>()
        .init_resource::<level::Difficulty>()
        .insert_resource(CurrentLevel(1))
        .insert_resource(StartLevel(start_level))
        .add_message::<LoadLevel>()
        .add_message::<Sfx>()
        .configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Think,
                GameSet::Animate,
                GameSet::Resolve,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .add_plugins((
            world::WorldPlugin,
            cat::CatPlugin,
            dog::DogPlugin,
            ui::UiPlugin,
            audio::AudioPlugin,
        ))
        .add_systems(
            Update,
            advance_phase.run_if(in_state(AppState::Playing)),
        );
    app.run();
}

/// Ticks the intro / wrong-yarn / level-complete timers and moves the game along.
fn advance_phase(
    time: Res<Time>,
    mut phase: ResMut<Phase>,
    mut level: ResMut<CurrentLevel>,
    mut load: MessageWriter<LoadLevel>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let delta = time.delta();
    let next = match &mut *phase {
        Phase::Play => return,
        Phase::Intro(timer) => timer.tick(delta).is_finished().then_some(Phase::Play),
        Phase::Wrong { timer, .. } => {
            if timer.tick(delta).is_finished() {
                load.write(LoadLevel);
                Some(Phase::Play)
            } else {
                None
            }
        }
        Phase::Complete(timer) => {
            if timer.tick(delta).is_finished() {
                if level.0 >= LEVEL_COUNT {
                    next_state.set(AppState::Victory);
                } else {
                    level.0 += 1;
                    load.write(LoadLevel);
                }
                Some(Phase::Play)
            } else {
                None
            }
        }
    };
    if let Some(next) = next {
        *phase = next;
    }
}

/// Debug helper: `--screenshot PATH [--after SECONDS]` saves a frame and quits.
#[derive(Resource)]
pub struct ScreenshotRequest {
    path: String,
    timer: Timer,
    taken: bool,
    /// Offscreen render target; window swapchains cannot be read back on every platform.
    pub target: Option<Handle<Image>>,
}

fn screenshot_request() -> Option<ScreenshotRequest> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1))?.clone();
    let after = args
        .iter()
        .position(|a| a == "--after")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(3.0);
    Some(ScreenshotRequest {
        path,
        timer: Timer::from_seconds(after, TimerMode::Once),
        taken: false,
        target: None,
    })
}

fn take_screenshot(time: Res<Time>, mut req: ResMut<ScreenshotRequest>, mut commands: Commands) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    if req.timer.tick(time.delta()).is_finished() {
        if !req.taken {
            req.taken = true;
            let path = req.path.clone();
            let shot = match &req.target {
                Some(image) => Screenshot::image(image.clone()),
                None => Screenshot::primary_window(),
            };
            commands.spawn(shot).observe(save_to_disk(path));
            req.timer = Timer::from_seconds(1.5, TimerMode::Once);
        } else {
            commands.write_message(AppExit::Success);
        }
    }
}

/// Debug helper: `--script "R,U,U,S"` plays keys (R/L/U/D = hop, S = swipe) every 0.5s.
#[derive(Resource)]
struct ScriptedInput {
    steps: Vec<char>,
    timer: Timer,
}

fn scripted_input(time: Res<Time>, mut script: ResMut<ScriptedInput>, mut keys: ResMut<ButtonInput<KeyCode>>, phase: Res<Phase>) {
    if !phase.is_play() || !script.timer.tick(time.delta()).is_finished() || script.steps.is_empty() {
        return;
    }
    script.timer.reset();
    let code = match script.steps.remove(0) {
        'R' => KeyCode::ArrowRight,
        'L' => KeyCode::ArrowLeft,
        'U' => KeyCode::ArrowUp,
        'D' => KeyCode::ArrowDown,
        _ => KeyCode::Space,
    };
    keys.press(code);
}

fn release_scripted_keys(mut keys: ResMut<ButtonInput<KeyCode>>) {
    for code in [KeyCode::ArrowRight, KeyCode::ArrowLeft, KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::Space] {
        if keys.pressed(code) {
            keys.release(code);
        }
    }
}
