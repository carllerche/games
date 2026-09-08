//! Flappy Bird clone built with Bevy (2D sprite renderer).
//!
//! All gameplay lives in `flappy_core`; this crate only draws things.
//!
//! Controls: Space, Up arrow, W, or left mouse click to flap.
//! Press Escape to quit.

use bevy::prelude::*;
use flappy_core::{palette, *};
use rand::Rng;

/// Z layers for the 2D scene.
mod layer {
    pub const CLOUDS: f32 = -5.0;
    pub const PIPES: f32 = 1.0;
    pub const GROUND: f32 = 5.0;
    pub const BIRD: f32 = 10.0;
    pub const TEXT: f32 = 20.0;
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct MessageText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(flappy_core::window("Flappy Bird")),
            ..default()
        }))
        .add_plugins(FlappyCorePlugin)
        .insert_resource(ClearColor(palette::SKY))
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
            )
                .after(CoreSystems),
        )
        .run();
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, flappy_core::fixed_projection()));

    // Decorative clouds.
    let mut rng = rand::rng();
    for _ in 0..6 {
        let x = rng.random_range(-WINDOW_WIDTH / 2.0..WINDOW_WIDTH / 2.0);
        let y = rng.random_range(40.0..WINDOW_HEIGHT / 2.0 - 40.0);
        let w = rng.random_range(60.0..120.0);
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.7), Vec2::new(w, w * 0.4)),
            Transform::from_xyz(x, y, layer::CLOUDS),
        ));
    }

    // Two ground tiles so the core can scroll them seamlessly.
    let ground = Ground::default();
    for i in 0..2 {
        let x = ground.tile_x(i);
        commands.spawn((
            ground,
            Sprite::from_color(palette::DIRT, Vec2::new(WINDOW_WIDTH, GROUND_HEIGHT)),
            Transform::from_xyz(x, GROUND_TOP - GROUND_HEIGHT / 2.0, layer::GROUND),
            children![
                // Grass strip along the top edge.
                (
                    Sprite::from_color(palette::GRASS, Vec2::new(WINDOW_WIDTH, 14.0)),
                    Transform::from_xyz(0.0, GROUND_HEIGHT / 2.0 - 7.0, 0.1),
                ),
                // Dashed stripe so the scrolling is visible.
                (
                    Sprite::from_color(palette::GRASS_DARK, Vec2::new(WINDOW_WIDTH * 0.5, 6.0)),
                    Transform::from_xyz(-WINDOW_WIDTH * 0.25, GROUND_HEIGHT / 2.0 - 20.0, 0.1),
                ),
            ],
        ));
    }

    // The bird: a yellow body with an eye, a beak, and a wing.
    commands.spawn((
        Bird::default(),
        Sprite::from_color(palette::BODY, BIRD_SIZE),
        Transform::from_xyz(BIRD_X, 0.0, layer::BIRD),
        children![
            (
                Sprite::from_color(palette::EYE, Vec2::new(11.0, 11.0)),
                Transform::from_xyz(9.0, 6.0, 0.1),
            ),
            (
                Sprite::from_color(palette::PUPIL, Vec2::new(5.0, 5.0)),
                Transform::from_xyz(11.0, 6.0, 0.2),
            ),
            (
                Sprite::from_color(palette::BEAK, Vec2::new(14.0, 8.0)),
                Transform::from_xyz(20.0, -2.0, 0.1),
            ),
            (
                Sprite::from_color(palette::WING, Vec2::new(16.0, 9.0)),
                Transform::from_xyz(-6.0, -4.0, 0.1),
            ),
        ],
    ));

    // Score readout at the top of the screen.
    commands.spawn((
        ScoreText,
        Text2d::new("0"),
        TextFont::from_font_size(56.0),
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Transform::from_xyz(0.0, WINDOW_HEIGHT / 2.0 - 70.0, layer::TEXT),
    ));

    // Centered message (instructions / game over).
    commands.spawn((
        MessageText,
        Text2d::new(""),
        TextFont::from_font_size(30.0),
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Transform::from_xyz(0.0, 40.0, layer::TEXT),
    ));
}

// ---------------------------------------------------------------------------
// Visuals for entities the core spawns
// ---------------------------------------------------------------------------

fn attach_pipe_visuals(
    mut commands: Commands,
    pipes: Query<(Entity, &PipeGeometry), Added<PipePair>>,
) {
    for (entity, g) in &pipes {
        let rim = |y: f32| {
            (
                Sprite::from_color(palette::PIPE_RIM, Vec2::new(PIPE_WIDTH + 8.0, 24.0)),
                Transform::from_xyz(0.0, y, 0.1),
            )
        };
        commands.spawn((
            ChildOf(entity),
            Sprite::from_color(palette::PIPE, Vec2::new(PIPE_WIDTH, g.top_height)),
            Transform::from_xyz(0.0, g.top_y, layer::PIPES),
            children![rim(-g.top_height / 2.0 + 12.0)],
        ));
        commands.spawn((
            ChildOf(entity),
            Sprite::from_color(palette::PIPE, Vec2::new(PIPE_WIDTH, g.bottom_height)),
            Transform::from_xyz(0.0, g.bottom_y, layer::PIPES),
            children![rim(g.bottom_height / 2.0 - 12.0)],
        ));
    }
}

fn attach_particle_visuals(
    mut commands: Commands,
    particles: Query<(Entity, &Particle, &Transform), Added<Particle>>,
) {
    for (entity, particle, transform) in &particles {
        let color = palette::DEBRIS[particle.color_index];
        // The core sizes debris through Transform scale, so the sprite is 1x1.
        commands.entity(entity).insert((
            Sprite::from_color(color, Vec2::ONE),
            transform.with_translation(transform.translation.with_z(layer::BIRD)),
        ));
    }
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

fn update_score_text(score: Res<Score>, mut text: Single<&mut Text2d, With<ScoreText>>) {
    if score.is_changed() {
        text.0 = score.current.to_string();
    }
}

fn show_ready_message(mut message: Single<&mut Text2d, With<MessageText>>) {
    message.0 = "Press SPACE or click\nto flap".into();
}

fn clear_message(mut message: Single<&mut Text2d, With<MessageText>>) {
    message.0.clear();
}

fn show_game_over_message(score: Res<Score>, mut message: Single<&mut Text2d, With<MessageText>>) {
    message.0 = format!(
        "GAME OVER\n\nScore: {}\nBest: {}\n\nPress SPACE or click\nto restart",
        score.current, score.best
    );
}
