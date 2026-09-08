//! Flappy Bird in 3D, cel-shaded in the style of The Wind Waker.
//!
//! All gameplay lives in `flappy_core` and is identical to the 2D game: the
//! action happens in the XY plane and this crate only decides how it looks.
//!
//! Controls: Space, Up arrow, W, or left mouse click to flap.
//! Press Escape to quit.

mod models;
mod toon;

use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*};
use flappy_core::{palette, *};
use models::{Builder, ModelKit, Wing};
use rand::Rng;
use std::f32::consts::TAU;
use toon::{ToonMaterial, ToonPlugin};

/// Camera distance chosen so the 45° vertical field of view shows roughly
/// the same 640 world units of height at Z = 0 as the 2D game does.
const CAMERA_DISTANCE: f32 = 790.0;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct MessageText;

/// Something that drifts left and wraps around, at its own speed.
#[derive(Component)]
struct Drift {
    speed: f32,
    wrap_x: f32,
}

/// Wing flap animation phase, in radians. Past one full cycle the wings idle.
#[derive(Resource, Default)]
struct WingPhase(f32);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(flappy_core::window("Flappy Bird 3D")),
            ..default()
        }))
        .add_plugins((FlappyCorePlugin, ToonPlugin, flappy_core::debug::DebugPlugin))
        .insert_resource(ClearColor(palette::SKY))
        .init_resource::<WingPhase>()
        .add_systems(PreStartup, init_model_kit)
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Ready), show_ready_message)
        .add_systems(OnEnter(GameState::Playing), clear_message)
        .add_systems(OnEnter(GameState::GameOver), show_game_over_message)
        .add_systems(
            Update,
            (
                attach_pipe_visuals,
                attach_particle_visuals,
                update_score_text,
                animate_wings,
                drift,
            )
                .after(CoreSystems),
        )
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn init_model_kit(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ModelKit::new(&mut meshes, &mut standard));
}

fn setup(
    mut commands: Commands,
    mut kit: ResMut<ModelKit>,
    mut toon: ResMut<Assets<ToonMaterial>>,
) {
    // Camera: in front of the play plane, raised a little so the tops of the
    // ground and pipes show, which sells the depth.
    commands.spawn((
        Camera3d::default(),
        // Keep the flat, saturated colors instead of filmic tonemapping.
        Tonemapping::None,
        Transform::from_xyz(0.0, 45.0, CAMERA_DISTANCE)
            .looking_at(Vec3::new(0.0, -15.0, 0.0), Vec3::Y),
    ));

    let mut rng = rand::rng();
    let mut b = Builder {
        commands: &mut commands,
        kit: &mut kit,
        toon: &mut toon,
    };

    // Backdrop: the sea and a scattering of far islands that parallax slowly.
    let sea = b
        .commands
        .spawn((Transform::default(), Visibility::default()))
        .id();
    b.sea(sea);
    for i in 0..5 {
        let x = -900.0 + i as f32 * 450.0 + rng.random_range(-80.0..80.0);
        let z = rng.random_range(-1500.0..-900.0);
        let island = b
            .commands
            .spawn((
                Drift {
                    speed: SCROLL_SPEED * 0.12,
                    wrap_x: 1200.0,
                },
                Transform::from_xyz(x, GROUND_TOP - GROUND_HEIGHT - 20.0, z),
                Visibility::default(),
            ))
            .id();
        b.island(island, &mut rng);
    }

    // Clouds behind the pipes.
    for _ in 0..7 {
        let x = rng.random_range(-WINDOW_WIDTH..WINDOW_WIDTH);
        let y = rng.random_range(60.0..CEILING + 60.0);
        let z = rng.random_range(-650.0..-350.0);
        let cloud = b
            .commands
            .spawn((
                Drift {
                    speed: rng.random_range(15.0..35.0),
                    wrap_x: WINDOW_WIDTH * 1.6,
                },
                Transform::from_xyz(x, y, z),
                Visibility::default(),
            ))
            .id();
        b.cloud(cloud, &mut rng);
    }

    // Ground tiles. The perspective camera sees past the window's half width
    // at the far edge of the ground, so cover that span (plus a margin) with
    // three tiles instead of the 2D game's two.
    let visible_half_width =
        WINDOW_WIDTH / 2.0 * (CAMERA_DISTANCE - models::GROUND_FAR_Z) / CAMERA_DISTANCE + 20.0;
    let ground = Ground::tiled(3, visible_half_width);
    for i in 0..3 {
        let tile = b
            .commands
            .spawn((
                ground,
                Transform::from_xyz(ground.tile_x(i), GROUND_TOP - GROUND_HEIGHT / 2.0, 0.0),
                Visibility::default(),
            ))
            .id();
        b.ground_tile(tile, &mut rng);
    }

    // The bird.
    let bird = b
        .commands
        .spawn((
            Bird::default(),
            Transform::from_xyz(BIRD_X, 0.0, 0.0),
            Visibility::default(),
        ))
        .id();
    b.bird(bird);

    // HUD.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            ScoreText,
            Text::new("0"),
            TextFont::from_font_size(56.0),
            TextColor(Color::WHITE),
            TextShadow::default(),
        )],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(140.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            MessageText,
            Text::new(""),
            TextFont::from_font_size(30.0),
            TextColor(Color::WHITE),
            TextLayout::justify(Justify::Center),
            TextShadow::default(),
        )],
    ));
}

// ---------------------------------------------------------------------------
// Visuals for entities the core spawns
// ---------------------------------------------------------------------------

fn attach_pipe_visuals(
    mut commands: Commands,
    mut kit: ResMut<ModelKit>,
    mut toon: ResMut<Assets<ToonMaterial>>,
    pipes: Query<(Entity, &PipeGeometry), Added<PipePair>>,
) {
    let mut b = Builder {
        commands: &mut commands,
        kit: &mut kit,
        toon: &mut toon,
    };
    for (entity, geometry) in &pipes {
        b.pipes(entity, geometry);
    }
}

fn attach_particle_visuals(
    mut commands: Commands,
    mut kit: ResMut<ModelKit>,
    mut toon: ResMut<Assets<ToonMaterial>>,
    particles: Query<(Entity, &Particle), Added<Particle>>,
) {
    let mut b = Builder {
        commands: &mut commands,
        kit: &mut kit,
        toon: &mut toon,
    };
    for (entity, particle) in &particles {
        b.debris(entity, palette::DEBRIS[particle.color_index]);
    }
}

// ---------------------------------------------------------------------------
// Animation
// ---------------------------------------------------------------------------

fn animate_wings(
    time: Res<Time>,
    mut flapped: MessageReader<Flapped>,
    mut phase: ResMut<WingPhase>,
    mut wings: Query<(&Wing, &mut Transform)>,
) {
    if !flapped.is_empty() {
        flapped.clear();
        phase.0 = 0.0;
    }
    phase.0 += time.delta_secs() * 18.0;

    // Wings rest raised so they show above the body from the camera's angle.
    const REST_ANGLE: f32 = 0.55;
    let angle = REST_ANGLE
        + if phase.0 < TAU {
            // One big beat right after a flap.
            phase.0.sin() * 0.9
        } else {
            // Then a gentle idle flutter.
            (time.elapsed_secs() * 5.0).sin() * 0.12
        };

    for (wing, mut transform) in &mut wings {
        transform.rotation = Quat::from_rotation_x(angle * wing.sign);
    }
}

fn drift(time: Res<Time>, mut drifters: Query<(&Drift, &mut Transform)>) {
    for (d, mut transform) in &mut drifters {
        transform.translation.x -= d.speed * time.delta_secs();
        if transform.translation.x < -d.wrap_x {
            transform.translation.x += d.wrap_x * 2.0;
        }
    }
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

fn update_score_text(score: Res<Score>, mut text: Single<&mut Text, With<ScoreText>>) {
    if score.is_changed() {
        text.0 = score.current.to_string();
    }
}

fn show_ready_message(mut message: Single<&mut Text, With<MessageText>>) {
    message.0 = "Press SPACE or click\nto flap".into();
}

fn clear_message(mut message: Single<&mut Text, With<MessageText>>) {
    message.0.clear();
}

fn show_game_over_message(score: Res<Score>, mut message: Single<&mut Text, With<MessageText>>) {
    message.0 = format!(
        "GAME OVER\n\nScore: {}\nBest: {}\n\nPress SPACE or click\nto restart",
        score.current, score.best
    );
}
