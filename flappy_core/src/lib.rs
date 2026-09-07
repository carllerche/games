//! Shared game logic for the Flappy Bird games.
//!
//! Everything that defines how the game *plays* lives here: constants, the
//! state machine, bird physics, pipe spawning, scrolling, scoring, collision,
//! and the explosion debris. Nothing in this crate draws anything. Each game
//! crate spawns the visuals for the entities this crate creates, so the 2D and
//! 3D versions share identical physics by construction.
//!
//! Gameplay happens in the XY plane. X scrolls right-to-left, Y is up. Game
//! crates are free to use Z however they like (layering in 2D, depth in 3D);
//! this crate never touches it.

use bevy::{
    math::bounding::{Aabb2d, IntersectsVolume},
    prelude::*,
};
use rand::Rng;

// ---------------------------------------------------------------------------
// Tuning constants
// ---------------------------------------------------------------------------

pub const WINDOW_WIDTH: f32 = 480.0;
pub const WINDOW_HEIGHT: f32 = 640.0;

pub const GROUND_HEIGHT: f32 = 90.0;
/// World-space y of the top edge of the ground.
pub const GROUND_TOP: f32 = -WINDOW_HEIGHT / 2.0 + GROUND_HEIGHT;
pub const CEILING: f32 = WINDOW_HEIGHT / 2.0;

pub const BIRD_X: f32 = -WINDOW_WIDTH / 4.0;
pub const BIRD_SIZE: Vec2 = Vec2::new(36.0, 26.0);
pub const GRAVITY: f32 = -1600.0;
pub const FLAP_VELOCITY: f32 = 460.0;
pub const MAX_FALL_SPEED: f32 = -800.0;

pub const SCROLL_SPEED: f32 = 170.0;
pub const PIPE_WIDTH: f32 = 72.0;
pub const PIPE_GAP: f32 = 175.0;
pub const PIPE_SPAWN_SECONDS: f32 = 1.55;
/// Keep the gap this far away from the ceiling and the ground.
pub const PIPE_GAP_MARGIN: f32 = 70.0;

pub const EXPLOSION_PARTICLES: usize = 60;
pub const PARTICLE_LIFETIME: f32 = 2.2;

/// How long after a crash before a flap can restart the game. Players are
/// usually still mashing the button when they hit a pipe, and without this
/// the very next press would skip past the explosion and the final score.
pub const RESTART_DELAY_SECONDS: f32 = 0.8;

// ---------------------------------------------------------------------------
// Palette shared by both renderers
// ---------------------------------------------------------------------------

pub mod palette {
    use bevy::prelude::Color;

    pub const SKY: Color = Color::srgb(0.44, 0.77, 0.81);
    pub const BODY: Color = Color::srgb(0.98, 0.84, 0.24);
    pub const WING: Color = Color::srgb(0.90, 0.71, 0.16);
    pub const BEAK: Color = Color::srgb(0.94, 0.43, 0.16);
    pub const EYE: Color = Color::WHITE;
    pub const PUPIL: Color = Color::BLACK;
    pub const PIPE: Color = Color::srgb(0.37, 0.79, 0.28);
    pub const PIPE_RIM: Color = Color::srgb(0.27, 0.63, 0.22);
    pub const DIRT: Color = Color::srgb(0.87, 0.85, 0.58);
    pub const GRASS: Color = Color::srgb(0.45, 0.75, 0.18);
    pub const GRASS_DARK: Color = Color::srgb(0.33, 0.59, 0.13);

    /// Colors the exploding bird breaks into, indexed by [`Particle::color_index`].
    ///
    /// [`Particle::color_index`]: crate::Particle::color_index
    pub const DEBRIS: [Color; 6] = [BODY, BODY, WING, BEAK, EYE, PUPIL];
}

// ---------------------------------------------------------------------------
// State, components, resources, messages
// ---------------------------------------------------------------------------

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Ready,
    Playing,
    GameOver,
}

/// The player. Game crates spawn exactly one of these with a `Transform`
/// and `Visibility`; this crate moves, tilts, hides, and resets it.
#[derive(Component, Default)]
pub struct Bird {
    pub velocity: f32,
}

/// Parent entity of a pipe pair. This crate spawns it (with colliders as
/// children); game crates attach visuals when they see it added.
#[derive(Component)]
pub struct PipePair {
    pub scored: bool,
}

/// Layout of a pipe pair, in the pair's local coordinates.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PipeGeometry {
    pub gap_center: f32,
    pub top_height: f32,
    pub top_y: f32,
    pub bottom_height: f32,
    pub bottom_y: f32,
}

/// An axis-aligned box the bird can crash into.
#[derive(Component, Debug, Clone, Copy)]
pub struct Collider {
    pub half_size: Vec2,
}

/// One tile of the scrolling ground, [`WINDOW_WIDTH`] wide. Game crates spawn
/// a row of tiles; this crate scrolls them left and wraps each one to the
/// back of the row once it has passed `wrap_x`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Ground {
    /// Once a tile's center is at or left of this, it jumps right by `period`.
    pub wrap_x: f32,
    /// Total width of the row of tiles.
    pub period: f32,
}

impl Ground {
    /// Layout for `tile_count` tiles that must keep the ground covered out to
    /// `visible_half_width` on both sides of center. A 2D camera sees exactly
    /// half the window; a perspective camera sees further at the far edge of
    /// the ground, so it needs a wider margin and more tiles.
    pub fn tiled(tile_count: usize, visible_half_width: f32) -> Self {
        let period = tile_count as f32 * WINDOW_WIDTH;
        // Wrap once the tile's right edge has left the visible area.
        let wrap_x = -(visible_half_width + WINDOW_WIDTH / 2.0);
        debug_assert!(
            period - WINDOW_WIDTH >= 2.0 * visible_half_width,
            "{tile_count} tiles cannot cover {visible_half_width} on each side"
        );
        Self { wrap_x, period }
    }

    /// Starting x for tile `index` so the row is centered on the screen and
    /// every tile is inside its wrap range.
    pub fn tile_x(&self, index: usize) -> f32 {
        self.wrap_x + WINDOW_WIDTH * (index as f32 + 1.0)
    }
}

impl Default for Ground {
    fn default() -> Self {
        Self::tiled(2, WINDOW_WIDTH / 2.0)
    }
}

/// A piece of debris from the exploded bird. This crate spawns these with a
/// `Transform` whose scale is the debris size; game crates attach a unit-size
/// visual (a 1x1 sprite, a 1x1x1 cube) when they see one added.
#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub spin: f32,
    pub size: f32,
    pub color_index: usize,
    pub lifetime: Timer,
}

#[derive(Resource, Default, Debug)]
pub struct Score {
    pub current: u32,
    pub best: u32,
}

#[derive(Resource)]
pub struct PipeTimer(pub Timer);

/// Counts down after a crash; restarting is ignored until it finishes.
#[derive(Resource)]
pub struct RestartDelay(pub Timer);

/// Sent whenever the bird flaps, so renderers can animate it.
#[derive(Message)]
pub struct Flapped;

/// All systems in this crate run inside this set. Order rendering systems
/// after it to see freshly spawned entities on the same frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoreSystems;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct FlappyCorePlugin;

impl Plugin for FlappyCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .insert_resource(PipeTimer(Timer::from_seconds(
                PIPE_SPAWN_SECONDS,
                TimerMode::Repeating,
            )))
            .insert_resource(RestartDelay(Timer::from_seconds(
                RESTART_DELAY_SECONDS,
                TimerMode::Once,
            )))
            .init_state::<GameState>()
            .add_message::<Flapped>()
            // Bevy runs the initial state transition before any startup
            // schedule, so nothing the games spawn exists yet. Re-enter the
            // initial state once startup is done so `OnEnter(Ready)` systems
            // (here and in the game crates) see the bird and the HUD.
            .add_systems(PostStartup, reenter_initial_state)
            .add_systems(OnEnter(GameState::Ready), enter_ready)
            .add_systems(OnEnter(GameState::Playing), enter_playing)
            .add_systems(OnEnter(GameState::GameOver), enter_game_over)
            .add_systems(
                Update,
                (
                    quit_on_escape,
                    scroll_ground.run_if(not(in_state(GameState::GameOver))),
                    bob_bird.run_if(in_state(GameState::Ready)),
                    start_on_flap.run_if(in_state(GameState::Ready)),
                    (
                        flap,
                        apply_bird_physics,
                        spawn_pipes,
                        scroll_pipes,
                        update_score,
                        check_collisions,
                    )
                        .chain()
                        .run_if(in_state(GameState::Playing)),
                    update_particles,
                    restart_on_flap.run_if(in_state(GameState::GameOver)),
                )
                    .in_set(CoreSystems),
            );
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (unit tested below)
// ---------------------------------------------------------------------------

/// Advance the bird's velocity and position by `dt` seconds.
/// Returns the new y position, clamped to the ceiling.
pub fn step_bird(bird: &mut Bird, y: f32, dt: f32) -> f32 {
    bird.velocity = (bird.velocity + GRAVITY * dt).max(MAX_FALL_SPEED);
    let y = y + bird.velocity * dt;
    let top_limit = CEILING - BIRD_SIZE.y / 2.0;
    if y > top_limit {
        bird.velocity = 0.0;
        top_limit
    } else {
        y
    }
}

/// Tilt angle (radians) for the given vertical velocity: nose up when rising,
/// nose down when falling.
pub fn bird_tilt(velocity: f32) -> f32 {
    (velocity / FLAP_VELOCITY * 0.5).clamp(-1.2, 0.45)
}

/// Vertical bob offset for the bird on the title screen.
pub fn bob_offset(elapsed_secs: f32) -> f32 {
    (elapsed_secs * 4.0).sin() * 8.0
}

/// Pick a random gap center that keeps both pipes visible.
pub fn random_gap_center(rng: &mut impl Rng) -> f32 {
    let min = GROUND_TOP + PIPE_GAP_MARGIN + PIPE_GAP / 2.0;
    let max = CEILING - PIPE_GAP_MARGIN - PIPE_GAP / 2.0;
    rng.random_range(min..max)
}

/// Compute pipe layout for a given gap center.
pub fn pipe_geometry(gap_center: f32) -> PipeGeometry {
    // Top pipe: from the ceiling down to the top of the gap.
    let top_height = CEILING - (gap_center + PIPE_GAP / 2.0);
    let top_y = CEILING - top_height / 2.0;
    // Bottom pipe: from the bottom of the gap down to the ground.
    let bottom_height = (gap_center - PIPE_GAP / 2.0) - GROUND_TOP;
    let bottom_y = GROUND_TOP + bottom_height / 2.0;
    PipeGeometry {
        gap_center,
        top_height,
        top_y,
        bottom_height,
        bottom_y,
    }
}

/// X where new pipe pairs appear (just off the right edge).
pub const PIPE_SPAWN_X: f32 = WINDOW_WIDTH / 2.0 + PIPE_WIDTH;
/// X past which pipe pairs are removed (just off the left edge).
pub const PIPE_DESPAWN_X: f32 = -WINDOW_WIDTH / 2.0 - PIPE_WIDTH;

/// True once a pipe pair at `pipe_x` has fully passed the bird.
pub fn pipe_passed(pipe_x: f32) -> bool {
    pipe_x + PIPE_WIDTH / 2.0 < BIRD_X
}

/// Does the bird at `bird_pos` hit the ground or any of the given boxes?
/// Boxes are `(center, half_size)` in world space.
pub fn bird_collides(bird_pos: Vec2, boxes: impl IntoIterator<Item = (Vec2, Vec2)>) -> bool {
    // Use a slightly smaller hitbox than the sprite so near misses feel fair.
    let bird_box = Aabb2d::new(bird_pos, BIRD_SIZE * 0.4);
    let hit_ground = bird_pos.y - BIRD_SIZE.y / 2.0 <= GROUND_TOP;
    hit_ground
        || boxes
            .into_iter()
            .any(|(center, half)| bird_box.intersects(&Aabb2d::new(center, half)))
}

/// Build the debris for an explosion at the bird's position.
pub fn explosion_particles(bird_velocity: f32, rng: &mut impl Rng) -> Vec<(Particle, Vec2)> {
    (0..EXPLOSION_PARTICLES)
        .map(|_| {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let speed = rng.random_range(150.0..650.0);
            // Carry a bit of the bird's own motion into the debris.
            let velocity = Vec2::from_angle(angle) * speed
                + Vec2::new(-SCROLL_SPEED * 0.3, bird_velocity * 0.4);
            let offset = Vec2::new(
                rng.random_range(-BIRD_SIZE.x / 2.0..BIRD_SIZE.x / 2.0),
                rng.random_range(-BIRD_SIZE.y / 2.0..BIRD_SIZE.y / 2.0),
            );
            let particle = Particle {
                velocity,
                spin: rng.random_range(-12.0..12.0),
                size: rng.random_range(4.0..11.0),
                color_index: rng.random_range(0..palette::DEBRIS.len()),
                lifetime: Timer::from_seconds(
                    PARTICLE_LIFETIME * rng.random_range(0.6..1.0),
                    TimerMode::Once,
                ),
            };
            (particle, offset)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// State transitions
// ---------------------------------------------------------------------------

fn reenter_initial_state(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Ready);
}

fn enter_ready(
    mut commands: Commands,
    pipes: Query<Entity, With<PipePair>>,
    particles: Query<Entity, With<Particle>>,
    mut bird: Single<(&mut Bird, &mut Transform, &mut Visibility)>,
    mut score: ResMut<Score>,
) {
    for entity in pipes.iter().chain(particles.iter()) {
        commands.entity(entity).despawn();
    }

    let (bird, transform, visibility) = &mut *bird;
    bird.velocity = 0.0;
    transform.translation.x = BIRD_X;
    transform.translation.y = 0.0;
    transform.rotation = Quat::IDENTITY;
    **visibility = Visibility::Visible;

    score.current = 0;
}

fn enter_playing(mut timer: ResMut<PipeTimer>) {
    timer.0.reset();
}

fn enter_game_over(
    mut commands: Commands,
    mut restart_delay: ResMut<RestartDelay>,
    mut bird: Single<(&Bird, &Transform, &mut Visibility)>,
) {
    restart_delay.0.reset();

    let (bird, transform, visibility) = &mut *bird;
    **visibility = Visibility::Hidden;

    let origin = transform.translation;
    let mut rng = rand::rng();
    for (particle, offset) in explosion_particles(bird.velocity, &mut rng) {
        let size = particle.size;
        commands.spawn((
            particle,
            Transform::from_translation(origin + offset.extend(0.0))
                .with_rotation(Quat::from_rotation_z(
                    rng.random_range(0.0..std::f32::consts::TAU),
                ))
                .with_scale(Vec3::splat(size)),
            Visibility::default(),
        ));
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

pub fn flap_pressed(keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
    keys.any_just_pressed([KeyCode::Space, KeyCode::ArrowUp, KeyCode::KeyW])
        || mouse.just_pressed(MouseButton::Left)
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// Ready state
// ---------------------------------------------------------------------------

fn bob_bird(time: Res<Time>, mut bird: Single<&mut Transform, With<Bird>>) {
    bird.translation.y = bob_offset(time.elapsed_secs());
    bird.rotation = Quat::IDENTITY;
}

fn start_on_flap(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut bird: Single<&mut Bird>,
    mut flapped: MessageWriter<Flapped>,
) {
    if flap_pressed(&keys, &mouse) {
        // Give the first flap immediately so the bird doesn't just drop.
        bird.velocity = FLAP_VELOCITY;
        flapped.write(Flapped);
        next_state.set(GameState::Playing);
    }
}

// ---------------------------------------------------------------------------
// Playing state
// ---------------------------------------------------------------------------

fn flap(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut bird: Single<&mut Bird>,
    mut flapped: MessageWriter<Flapped>,
) {
    if flap_pressed(&keys, &mouse) {
        bird.velocity = FLAP_VELOCITY;
        flapped.write(Flapped);
    }
}

fn apply_bird_physics(time: Res<Time>, mut bird: Single<(&mut Bird, &mut Transform)>) {
    let (bird, transform) = &mut *bird;
    transform.translation.y = step_bird(bird, transform.translation.y, time.delta_secs());
    transform.rotation = Quat::from_rotation_z(bird_tilt(bird.velocity));
}

fn spawn_pipes(mut commands: Commands, time: Res<Time>, mut timer: ResMut<PipeTimer>) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let geometry = pipe_geometry(random_gap_center(&mut rand::rng()));
    let half_width = PIPE_WIDTH / 2.0;

    commands.spawn((
        PipePair { scored: false },
        geometry,
        Transform::from_xyz(PIPE_SPAWN_X, 0.0, 0.0),
        Visibility::default(),
        children![
            (
                Collider {
                    half_size: Vec2::new(half_width, geometry.top_height / 2.0),
                },
                Transform::from_xyz(0.0, geometry.top_y, 0.0),
            ),
            (
                Collider {
                    half_size: Vec2::new(half_width, geometry.bottom_height / 2.0),
                },
                Transform::from_xyz(0.0, geometry.bottom_y, 0.0),
            ),
        ],
    ));
}

fn scroll_pipes(
    mut commands: Commands,
    time: Res<Time>,
    mut pipes: Query<(Entity, &mut Transform), With<PipePair>>,
) {
    for (entity, mut transform) in &mut pipes {
        transform.translation.x -= SCROLL_SPEED * time.delta_secs();
        if transform.translation.x < PIPE_DESPAWN_X {
            commands.entity(entity).despawn();
        }
    }
}

fn scroll_ground(time: Res<Time>, mut tiles: Query<(&Ground, &mut Transform)>) {
    for (ground, mut transform) in &mut tiles {
        transform.translation.x -= SCROLL_SPEED * time.delta_secs();
        if transform.translation.x <= ground.wrap_x {
            transform.translation.x += ground.period;
        }
    }
}

fn update_score(mut pipes: Query<(&mut PipePair, &Transform)>, mut score: ResMut<Score>) {
    for (mut pair, transform) in &mut pipes {
        if !pair.scored && pipe_passed(transform.translation.x) {
            pair.scored = true;
            score.current += 1;
            score.best = score.best.max(score.current);
        }
    }
}

fn check_collisions(
    bird: Single<&Transform, With<Bird>>,
    colliders: Query<(&GlobalTransform, &Collider)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let boxes = colliders
        .iter()
        .map(|(global, collider)| (global.translation().truncate(), collider.half_size));
    if bird_collides(bird.translation.truncate(), boxes) {
        next_state.set(GameState::GameOver);
    }
}

// ---------------------------------------------------------------------------
// Game over state
// ---------------------------------------------------------------------------

fn restart_on_flap(
    time: Res<Time>,
    mut restart_delay: ResMut<RestartDelay>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Tick first so presses during the lockout are dropped, not queued.
    if restart_delay.0.tick(time.delta()).is_finished() && flap_pressed(&keys, &mouse) {
        next_state.set(GameState::Ready);
    }
}

// ---------------------------------------------------------------------------
// Explosion debris
// ---------------------------------------------------------------------------

/// Move, spin, bounce, shrink, and eventually despawn explosion debris.
fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform) in &mut particles {
        if particle.lifetime.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        particle.velocity.y += GRAVITY * 0.6 * dt;
        transform.translation += (particle.velocity * dt).extend(0.0);
        transform.rotate_z(particle.spin * dt);

        // Bounce off the ground, losing energy and spin each time.
        let half = transform.scale.y / 2.0;
        if transform.translation.y - half < GROUND_TOP && particle.velocity.y < 0.0 {
            transform.translation.y = GROUND_TOP + half;
            particle.velocity.y = -particle.velocity.y * 0.45;
            particle.velocity.x *= 0.7;
            particle.spin *= 0.5;
        }

        // Shrink away over the last 40% of the lifetime.
        let remaining = particle.lifetime.fraction_remaining();
        let shrink = (remaining / 0.4).clamp(0.0, 1.0);
        transform.scale = Vec3::splat((particle.size * shrink).max(0.01));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_pulls_bird_down() {
        let mut bird = Bird { velocity: 0.0 };
        let y = step_bird(&mut bird, 0.0, 0.1);
        assert!(y < 0.0);
        assert!(bird.velocity < 0.0);
    }

    #[test]
    fn fall_speed_is_capped() {
        let mut bird = Bird { velocity: 0.0 };
        let mut y = 0.0;
        for _ in 0..100 {
            y = step_bird(&mut bird, y, 0.05);
        }
        assert_eq!(bird.velocity, MAX_FALL_SPEED);
    }

    #[test]
    fn bird_cannot_leave_through_the_ceiling() {
        let mut bird = Bird {
            velocity: FLAP_VELOCITY,
        };
        let y = step_bird(&mut bird, CEILING, 0.016);
        assert!(y <= CEILING - BIRD_SIZE.y / 2.0);
        assert_eq!(bird.velocity, 0.0);
    }

    #[test]
    fn tilt_follows_velocity() {
        assert!(bird_tilt(FLAP_VELOCITY) > 0.0);
        assert!(bird_tilt(MAX_FALL_SPEED) < 0.0);
        // Extreme velocities are clamped to the nose-up / nose-down limits.
        assert_eq!(bird_tilt(10_000.0), 0.45);
        assert_eq!(bird_tilt(-10_000.0), -1.2);
    }

    #[test]
    fn pipe_geometry_spans_ceiling_to_ground() {
        let mut rng = rand::rng();
        for _ in 0..200 {
            let g = pipe_geometry(random_gap_center(&mut rng));
            let top_bottom_edge = g.top_y - g.top_height / 2.0;
            let top_top_edge = g.top_y + g.top_height / 2.0;
            let bottom_top_edge = g.bottom_y + g.bottom_height / 2.0;
            let bottom_bottom_edge = g.bottom_y - g.bottom_height / 2.0;

            assert!((top_top_edge - CEILING).abs() < 1e-3);
            assert!((bottom_bottom_edge - GROUND_TOP).abs() < 1e-3);
            assert!((top_bottom_edge - bottom_top_edge - PIPE_GAP).abs() < 1e-3);
            assert!(g.top_height >= PIPE_GAP_MARGIN - 1e-3);
            assert!(g.bottom_height >= PIPE_GAP_MARGIN - 1e-3);
        }
    }

    #[test]
    fn bird_fits_through_the_gap() {
        let g = pipe_geometry(0.0);
        let boxes = [
            (
                Vec2::new(BIRD_X, g.top_y),
                Vec2::new(PIPE_WIDTH / 2.0, g.top_height / 2.0),
            ),
            (
                Vec2::new(BIRD_X, g.bottom_y),
                Vec2::new(PIPE_WIDTH / 2.0, g.bottom_height / 2.0),
            ),
        ];
        assert!(!bird_collides(Vec2::new(BIRD_X, 0.0), boxes));
        assert!(bird_collides(Vec2::new(BIRD_X, g.top_y), boxes));
        assert!(bird_collides(Vec2::new(BIRD_X, GROUND_TOP), []));
    }

    #[test]
    fn scoring_happens_after_the_pipe_clears_the_bird() {
        assert!(!pipe_passed(BIRD_X));
        assert!(!pipe_passed(BIRD_X - PIPE_WIDTH / 2.0));
        assert!(pipe_passed(BIRD_X - PIPE_WIDTH / 2.0 - 1.0));
    }

    /// Scroll a row of tiles for a long time and check the visible span is
    /// always covered with no gaps.
    fn assert_ground_stays_covered(tile_count: usize, visible_half_width: f32) {
        let ground = Ground::tiled(tile_count, visible_half_width);
        let mut xs: Vec<f32> = (0..tile_count).map(|i| ground.tile_x(i)).collect();
        let dt = 1.0 / 60.0;
        for _ in 0..(60 * 60) {
            for x in &mut xs {
                *x -= SCROLL_SPEED * dt;
                if *x <= ground.wrap_x {
                    *x += ground.period;
                }
            }
            let mut sorted = xs.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let left = sorted[0] - WINDOW_WIDTH / 2.0;
            let right = sorted[sorted.len() - 1] + WINDOW_WIDTH / 2.0;
            assert!(left <= -visible_half_width, "gap on the left: {left}");
            assert!(right >= visible_half_width, "gap on the right: {right}");
            for pair in sorted.windows(2) {
                assert!(
                    (pair[1] - pair[0] - WINDOW_WIDTH).abs() < 1e-2,
                    "tiles not contiguous"
                );
            }
        }
    }

    #[test]
    fn two_ground_tiles_cover_a_2d_camera() {
        assert_ground_stays_covered(2, WINDOW_WIDTH / 2.0);
    }

    #[test]
    fn three_ground_tiles_cover_a_wide_perspective_view() {
        assert_ground_stays_covered(3, 360.0);
    }

    /// A headless app with the core plugin and a bird, for driving the state
    /// machine without a window.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::state::app::StatesPlugin,
            FlappyCorePlugin,
        ));
        // Bare input resources, so tests control presses directly without
        // the input plugin clearing them each frame.
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>();
        app.world_mut().spawn((
            Bird::default(),
            Transform::from_xyz(BIRD_X, 0.0, 0.0),
            Visibility::default(),
        ));
        app.update();
        app
    }

    fn state(app: &App) -> GameState {
        *app.world().resource::<State<GameState>>().get()
    }

    /// Tap space for one frame, then run one more frame so any state change
    /// it caused is applied (transitions run at the start of the next frame).
    fn press_space(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
    }

    fn crash(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::GameOver);
        app.update();
        assert_eq!(state(app), GameState::GameOver);
    }

    #[test]
    fn mashing_the_button_during_a_crash_does_not_skip_game_over() {
        let mut app = headless_app();
        press_space(&mut app);
        assert_eq!(state(&app), GameState::Playing);

        crash(&mut app);
        let debris = app
            .world_mut()
            .query::<&Particle>()
            .iter(app.world())
            .count();
        assert_eq!(debris, EXPLOSION_PARTICLES);

        // Presses right after the crash are ignored.
        for _ in 0..5 {
            press_space(&mut app);
            assert_eq!(state(&app), GameState::GameOver);
        }
    }

    #[test]
    fn restart_works_once_the_delay_has_passed() {
        let mut app = headless_app();
        press_space(&mut app);
        crash(&mut app);

        app.world_mut()
            .resource_mut::<RestartDelay>()
            .0
            .tick(std::time::Duration::from_secs_f32(RESTART_DELAY_SECONDS));
        press_space(&mut app);
        assert_eq!(state(&app), GameState::Ready);

        let debris = app
            .world_mut()
            .query::<&Particle>()
            .iter(app.world())
            .count();
        assert_eq!(debris, 0);
        let visibility = app
            .world_mut()
            .query_filtered::<&Visibility, With<Bird>>()
            .single(app.world())
            .unwrap();
        assert_eq!(*visibility, Visibility::Visible);
    }

    #[test]
    fn explosion_makes_the_right_amount_of_debris() {
        let debris = explosion_particles(0.0, &mut rand::rng());
        assert_eq!(debris.len(), EXPLOSION_PARTICLES);
        assert!(
            debris
                .iter()
                .all(|(p, _)| p.color_index < palette::DEBRIS.len())
        );
    }
}
