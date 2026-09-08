//! The player's cat: its blocky model, keyboard and mouse control, hopping,
//! long-jumping across gaps, swiping toys and dogs, falling, and being slimed.

use crate::dog::{Dog, Hurt};
use crate::grid::{Dir, FLOOR_Y, Facing, Grid, GridPos, HopPlan, cell_to_world, level_y};
use crate::world::{CameraRig, GameAssets, HOP_ARC, Particle, Squash, Toy, Yarn, fling, spawn_poof};
use crate::{GameSet, LevelEntity, Phase, Sfx, in_play, level::YarnColor};
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Landed>()
            .add_systems(
                Update,
                (
                    cat_input.in_set(GameSet::Input).run_if(in_play),
                    (animate_hops, animate_swipes, animate_wiggle, falling, face_direction, wag_tail).in_set(GameSet::Animate),
                    (cat_landed, slimy, slimy_skin).in_set(GameSet::Resolve),
                ),
            );
    }
}

#[derive(Component)]
pub struct Cat;

/// Cat body parts that turn green when slimed.
#[derive(Component)]
pub struct CatSkin {
    stripe: bool,
}

#[derive(Component)]
pub struct SwipePaw;

#[derive(Component)]
pub struct Tail;

/// Licked by a dog: hops are slow until the timer runs out.
#[derive(Component)]
pub struct Slimy {
    pub timer: Timer,
    drip: Timer,
}

/// An in-progress hop between two world positions. Used by the cat and the dogs.
#[derive(Component)]
pub struct Hop {
    pub from: Vec3,
    pub to: Vec3,
    pub t: f32,
    pub duration: f32,
    pub arc: f32,
    /// The cell we land on, or `None` if this hop goes off an edge.
    pub target: Option<IVec2>,
}

/// A hop finished. `cell` is `None` when the hopper left the platforms.
#[derive(Message)]
pub struct Landed {
    pub entity: Entity,
    pub cell: Option<IVec2>,
}

#[derive(Component)]
pub struct Falling {
    velocity: f32,
    respawn: IVec2,
}

#[derive(Component)]
pub struct Swipe {
    t: f32,
    dir: Dir,
    hit_done: bool,
}

/// A little "nope" shake when a hop is not allowed.
#[derive(Component)]
pub struct Wiggle {
    t: f32,
}

const HOP_TIME: f32 = 0.22;
const LONG_HOP_TIME: f32 = 0.42;
const SWIPE_TIME: f32 = 0.3;
const SLIME_TIME: f32 = 4.0;
const SLIME_SLOWDOWN: f32 = 2.2;

pub fn spawn_cat(commands: &mut Commands, assets: &GameAssets, top: Vec3, cell: IVec2) {
    let fur = assets.cat_fur.clone();
    let stripe = assets.cat_stripe.clone();
    commands
        .spawn((
            LevelEntity,
            Cat,
            GridPos(cell),
            Facing(Dir::East),
            Transform {
                translation: top,
                rotation: Quat::from_rotation_y(Dir::East.yaw()),
                ..default()
            },
            Visibility::default(),
        ))
        .with_children(|p| {
            // Body.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(fur.clone()),
                CatSkin { stripe: false },
                Transform {
                    translation: Vec3::new(0.0, 0.34, 0.05),
                    scale: Vec3::new(0.52, 0.42, 0.78),
                    ..default()
                },
            ));
            // Belly.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.cat_belly.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.2, 0.05),
                    scale: Vec3::new(0.36, 0.16, 0.6),
                    ..default()
                },
            ));
            // Back stripes.
            for z in [-0.18, 0.02, 0.22] {
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(stripe.clone()),
                    CatSkin { stripe: true },
                    Transform {
                        translation: Vec3::new(0.0, 0.56, z),
                        scale: Vec3::new(0.4, 0.04, 0.08),
                        ..default()
                    },
                ));
            }
            // Head.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(fur.clone()),
                CatSkin { stripe: false },
                Transform {
                    translation: Vec3::new(0.0, 0.62, -0.42),
                    scale: Vec3::new(0.5, 0.42, 0.42),
                    ..default()
                },
            ));
            // Ears.
            for side in [-1.0, 1.0] {
                p.spawn((
                    Mesh3d(assets.cone.clone()),
                    MeshMaterial3d(fur.clone()),
                    CatSkin { stripe: false },
                    Transform {
                        translation: Vec3::new(side * 0.17, 0.92, -0.42),
                        scale: Vec3::new(0.18, 0.24, 0.12),
                        ..default()
                    },
                ));
                p.spawn((
                    Mesh3d(assets.cone.clone()),
                    MeshMaterial3d(assets.pink.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.17, 0.91, -0.44),
                        scale: Vec3::new(0.09, 0.14, 0.06),
                        ..default()
                    },
                ));
                // Eyes.
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.white.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.13, 0.68, -0.62),
                        scale: Vec3::splat(0.13),
                        ..default()
                    },
                ));
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.black.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.13, 0.68, -0.68),
                        scale: Vec3::splat(0.06),
                        ..default()
                    },
                ));
                // Whiskers.
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.white.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.3, 0.55, -0.6),
                        scale: Vec3::new(0.25, 0.015, 0.015),
                        ..default()
                    },
                ));
            }
            // Nose.
            p.spawn((
                Mesh3d(assets.sphere.clone()),
                MeshMaterial3d(assets.pink.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.56, -0.64),
                    scale: Vec3::splat(0.08),
                    ..default()
                },
            ));
            // Legs. The front-right one is the swiping paw.
            for (x, z, paw) in [(-0.17, -0.25, false), (0.17, -0.25, true), (-0.17, 0.3, false), (0.17, 0.3, false)] {
                let mut leg = p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(fur.clone()),
                    CatSkin { stripe: false },
                    Transform {
                        translation: Vec3::new(x, 0.12, z),
                        scale: Vec3::new(0.16, 0.24, 0.16),
                        ..default()
                    },
                ));
                if paw {
                    leg.insert(SwipePaw);
                }
            }
            // Tail.
            p.spawn((
                Mesh3d(assets.cylinder.clone()),
                MeshMaterial3d(stripe.clone()),
                CatSkin { stripe: true },
                Tail,
                Transform {
                    translation: Vec3::new(0.0, 0.55, 0.52),
                    rotation: Quat::from_rotation_x(0.5),
                    scale: Vec3::new(0.09, 0.5, 0.09),
                },
            ));
        });
}

/// What the player asked for this frame.
#[derive(Clone, Copy, Debug)]
enum Intent {
    Move(Dir),
    Swipe(Option<Dir>),
}

#[allow(clippy::too_many_arguments)]
fn cat_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<CameraRig>>,
    grid: Res<Grid>,
    mut cat: Query<(Entity, &GridPos, &mut Facing, &Transform, Option<&Slimy>), (With<Cat>, Without<Hop>, Without<Falling>, Without<Swipe>)>,
    blockers: Query<(&GridPos, Option<&Toy>, Option<&Dog>), Without<Cat>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let Ok((entity, pos, mut facing, transform, slimy)) = cat.single_mut() else {
        return;
    };

    let mut intent = None;
    let key_dirs = [
        (Dir::North, [KeyCode::ArrowUp, KeyCode::KeyW]),
        (Dir::South, [KeyCode::ArrowDown, KeyCode::KeyS]),
        (Dir::West, [KeyCode::ArrowLeft, KeyCode::KeyA]),
        (Dir::East, [KeyCode::ArrowRight, KeyCode::KeyD]),
    ];
    for (dir, codes) in key_dirs {
        if codes.iter().any(|c| keys.just_pressed(*c)) {
            intent = Some(Intent::Move(dir));
        }
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        intent = Some(Intent::Swipe(None));
    }

    if intent.is_none() && mouse.just_pressed(MouseButton::Left) {
        let (camera, cam_tf) = *camera;
        if let Some(cursor) = window.cursor_position()
            && let Some(cell) = pick_cell(camera, cam_tf, cursor, &grid)
        {
            let delta = cell - pos.0;
            let occupied = blockers.iter().any(|(p, toy, dog)| p.0 == cell && (toy.is_some() || dog.is_some()));
            intent = match Dir::from_offset(delta) {
                Some(dir) if occupied => Some(Intent::Swipe(Some(dir))),
                Some(dir) => Some(Intent::Move(dir)),
                None if delta == IVec2::ZERO => Some(Intent::Swipe(None)),
                None => Dir::toward(delta).map(Intent::Move),
            };
        }
    }

    let Some(intent) = intent else {
        return;
    };

    let slow = if slimy.is_some() { SLIME_SLOWDOWN } else { 1.0 };
    let here = pos.0;
    let here_h = grid.height(here).unwrap_or(0);
    let from = transform.translation;

    match intent {
        Intent::Swipe(dir) => {
            let dir = dir.unwrap_or(facing.0);
            facing.0 = dir;
            commands.entity(entity).insert(Swipe {
                t: 0.0,
                dir,
                hit_done: false,
            });
            sfx.write(Sfx::Swipe);
        }
        Intent::Move(dir) => {
            facing.0 = dir;
            let is_blocked = |cell: IVec2| blockers.iter().any(|(p, toy, dog)| p.0 == cell && (toy.is_some() || dog.is_some()));

            let plan = grid.plan_hop(here, dir);
            let landing = match plan {
                HopPlan::Step(cell) | HopPlan::Leap(cell) => Some(cell),
                _ => None,
            };
            if landing.is_some_and(is_blocked) || plan == HopPlan::TooHigh {
                // Something is in the way, or the ledge is too high: nudge against it.
                commands.entity(entity).insert(Wiggle { t: 0.0 });
                sfx.write(Sfx::Bump);
                return;
            }
            let (to, duration, arc, target, sound) = match plan {
                HopPlan::Step(cell) => {
                    let h = grid.height(cell).unwrap_or(here_h);
                    (cell_to_world(cell, h), HOP_TIME, HOP_ARC + (h - here_h).max(0) as f32 * 0.4, Some(cell), Sfx::Hop)
                }
                HopPlan::Leap(cell) => {
                    let h = grid.height(cell).unwrap_or(here_h);
                    (cell_to_world(cell, h), LONG_HOP_TIME, HOP_ARC * 2.2, Some(cell), Sfx::LongHop)
                }
                // Nothing there: hop off the edge and fall.
                HopPlan::Edge | HopPlan::TooHigh => (
                    from + Vec3::new(dir.offset().x as f32, 0.0, dir.offset().y as f32),
                    HOP_TIME,
                    HOP_ARC,
                    None,
                    Sfx::Hop,
                ),
            };
            commands.entity(entity).insert(Hop {
                from,
                to,
                t: 0.0,
                duration: duration * slow,
                arc,
                target,
            });
            sfx.write(sound);
        }
    }
}

/// Finds the tile under the mouse cursor. Higher tiles are tested first
/// because they occlude lower ones.
fn pick_cell(camera: &Camera, cam_tf: &GlobalTransform, cursor: Vec2, grid: &Grid) -> Option<IVec2> {
    let ray = camera.viewport_to_world(cam_tf, cursor).ok()?;
    for h in (0..=grid.max_height).rev() {
        let plane_y = level_y(h);
        let point = ray.plane_intersection_point(Vec3::new(0.0, plane_y, 0.0), InfinitePlane3d::new(Vec3::Y))?;
        let cell = IVec2::new(point.x.round() as i32, point.z.round() as i32);
        if grid.height(cell) == Some(h) {
            return Some(cell);
        }
    }
    None
}

fn animate_hops(
    time: Res<Time>,
    mut commands: Commands,
    mut hoppers: Query<(Entity, &mut Transform, &mut Hop, &mut GridPos)>,
    mut landed: MessageWriter<Landed>,
) {
    for (entity, mut tf, mut hop, mut pos) in &mut hoppers {
        hop.t += time.delta_secs();
        let s = (hop.t / hop.duration).min(1.0);
        let eased = s;
        tf.translation = hop.from.lerp(hop.to, eased) + Vec3::Y * hop.arc * (PI * s).sin();
        // Stretch while airborne.
        let air = (PI * s).sin();
        tf.scale = Vec3::new(1.0 - 0.12 * air, 1.0 + 0.2 * air, 1.0 - 0.12 * air);
        if s >= 1.0 {
            tf.translation = hop.to;
            tf.scale = Vec3::ONE;
            if let Some(cell) = hop.target {
                pos.0 = cell;
            }
            commands.entity(entity).remove::<Hop>().insert(Squash { t: 0.0 });
            landed.write(Landed {
                entity,
                cell: hop.target,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cat_landed(
    mut commands: Commands,
    mut landed: MessageReader<Landed>,
    assets: Res<GameAssets>,
    cat: Query<(&GridPos, &Transform), With<Cat>>,
    yarn: Query<(Entity, &GridPos, &Yarn, &Transform)>,
    mut phase: ResMut<Phase>,
    mut sfx: MessageWriter<Sfx>,
) {
    for event in landed.read() {
        let Ok((pos, transform)) = cat.get(event.entity) else {
            continue;
        };
        match event.cell {
            None => {
                commands.entity(event.entity).remove::<Squash>().insert(Falling {
                    velocity: 0.0,
                    respawn: pos.0,
                });
                sfx.write(Sfx::Fall);
            }
            Some(cell) => {
                if let Some((yarn_entity, _, yarn, yarn_tf)) = yarn.iter().find(|(_, p, _, _)| p.0 == cell) {
                    if !phase.is_play() {
                        continue;
                    }
                    let at = yarn_tf.translation;
                    match yarn.0 {
                        YarnColor::Purple => {
                            spawn_poof(&mut commands, &assets, at, 14, assets.sparkle.clone(), 4.0);
                            commands.entity(yarn_entity).despawn();
                            *phase = Phase::Complete(Timer::from_seconds(2.2, TimerMode::Once));
                            sfx.write(Sfx::Collect);
                        }
                        other => {
                            spawn_poof(&mut commands, &assets, at, 8, assets.yarn[&other].clone(), 3.0);
                            commands.entity(yarn_entity).despawn();
                            *phase = Phase::Wrong {
                                timer: Timer::from_seconds(2.2, TimerMode::Once),
                                color: other,
                            };
                            sfx.write(Sfx::Wrong);
                        }
                    }
                }
                let _ = transform;
            }
        }
    }
}

fn falling(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    grid: Res<Grid>,
    mut q: Query<(Entity, &mut Transform, &mut Falling, &mut GridPos)>,
    mut sfx: MessageWriter<Sfx>,
) {
    for (entity, mut tf, mut fall, mut pos) in &mut q {
        fall.velocity -= 30.0 * time.delta_secs();
        tf.translation.y += fall.velocity * time.delta_secs();
        tf.rotate_local_x(3.0 * time.delta_secs());
        if tf.translation.y <= FLOOR_Y {
            let landing = Vec3::new(tf.translation.x, FLOOR_Y, tf.translation.z);
            spawn_poof(&mut commands, &assets, landing, 10, assets.poof.clone(), 3.0);
            sfx.write(Sfx::Bump);
            // Reappear where we last stood, with a puff.
            let back = grid.top_of(fall.respawn).unwrap_or(landing);
            pos.0 = fall.respawn;
            tf.translation = back;
            tf.rotation = Quat::IDENTITY;
            spawn_poof(&mut commands, &assets, back + Vec3::Y * 0.3, 8, assets.poof.clone(), 2.0);
            commands.entity(entity).remove::<Falling>().insert(Squash { t: 0.0 });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn animate_swipes(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut cat: Query<(Entity, &GridPos, &mut Swipe, &Children, Option<&Slimy>), With<Cat>>,
    mut paws: Query<&mut Transform, (With<SwipePaw>, Without<Toy>, Without<Dog>)>,
    toys: Query<(Entity, &GridPos, &Transform), With<Toy>>,
    mut dogs: Query<(Entity, &GridPos, &Transform, &mut Dog)>,
    mut sfx: MessageWriter<Sfx>,
) {
    for (entity, pos, mut swipe, children, slimy) in &mut cat {
        let slow = if slimy.is_some() { 1.5 } else { 1.0 };
        swipe.t += time.delta_secs() / slow;
        let s = (swipe.t / SWIPE_TIME).min(1.0);

        // Paw shoots forward and back.
        let reach = (PI * s).sin();
        for child in children.iter() {
            if let Ok(mut paw) = paws.get_mut(child) {
                paw.translation = Vec3::new(0.17, 0.12 + 0.35 * reach, -0.25 - 0.55 * reach);
                paw.rotation = Quat::from_rotation_x(reach * FRAC_PI_2);
                paw.scale = Vec3::new(0.16, 0.24 + 0.2 * reach, 0.16);
            }
        }

        if !swipe.hit_done && s >= 0.4 {
            swipe.hit_done = true;
            let target = pos.0 + swipe.dir.offset();
            let push = Vec3::new(swipe.dir.offset().x as f32, 0.0, swipe.dir.offset().y as f32);
            if let Some((toy, _, toy_tf)) = toys.iter().find(|(_, p, _)| p.0 == target) {
                fling(&mut commands, toy, push, 5.0);
                spawn_poof(&mut commands, &assets, toy_tf.translation + Vec3::Y * 0.3, 6, assets.poof.clone(), 2.5);
                sfx.write(Sfx::Hit);
            }
            if let Some((dog_entity, _, dog_tf, mut dog)) = dogs.iter_mut().find(|(_, p, _, _)| p.0 == target) {
                dog.hp -= 1;
                if dog.hp <= 0 {
                    // The dog runs off yelping, tumbling away from the cat.
                    commands.entity(dog_entity).remove::<(Dog, Hop)>();
                    fling(&mut commands, dog_entity, push, 6.0);
                    spawn_poof(&mut commands, &assets, dog_tf.translation + Vec3::Y * 0.4, 10, assets.poof.clone(), 3.0);
                    sfx.write(Sfx::DogGone);
                } else {
                    commands.entity(dog_entity).insert(Hurt { t: 0.0 });
                    spawn_poof(&mut commands, &assets, dog_tf.translation + Vec3::Y * 0.5, 4, assets.poof.clone(), 2.0);
                    sfx.write(Sfx::Yelp);
                }
            }
        }

        if s >= 1.0 {
            commands.entity(entity).remove::<Swipe>();
        }
    }
}

fn animate_wiggle(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut Transform, &mut Wiggle, &Facing)>) {
    for (e, mut tf, mut w, facing) in &mut q {
        w.t += time.delta_secs();
        let d = 0.25;
        let base = Quat::from_rotation_y(facing.0.yaw());
        if w.t >= d {
            tf.rotation = base;
            commands.entity(e).remove::<Wiggle>();
            continue;
        }
        tf.rotation = base * Quat::from_rotation_y((w.t * 40.0).sin() * 0.25);
    }
}

/// Smoothly turn actors toward the direction they face.
fn face_direction(time: Res<Time>, mut q: Query<(&mut Transform, &Facing), (Without<Wiggle>, Without<Falling>, Without<Particle>)>) {
    let k = 1.0 - (-time.delta_secs() * 18.0).exp();
    for (mut tf, facing) in &mut q {
        let goal = Quat::from_rotation_y(facing.0.yaw());
        tf.rotation = tf.rotation.slerp(goal, k);
    }
}

fn wag_tail(time: Res<Time>, mut tails: Query<&mut Transform, With<Tail>>) {
    let t = time.elapsed_secs();
    for mut tf in &mut tails {
        tf.rotation = Quat::from_rotation_x(0.5) * Quat::from_rotation_z((t * 4.0).sin() * 0.35);
    }
}

/// Makes the cat slimy (or refreshes the timer if it already is).
pub fn slime(commands: &mut Commands, cat: Entity) {
    commands.entity(cat).insert(Slimy {
        timer: Timer::from_seconds(SLIME_TIME, TimerMode::Once),
        drip: Timer::from_seconds(0.25, TimerMode::Repeating),
    });
}

fn slimy(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut q: Query<(Entity, &mut Slimy, &Transform), With<Cat>>,
) {
    for (entity, mut slimy, tf) in &mut q {
        if slimy.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).remove::<Slimy>();
            continue;
        }
        if slimy.drip.tick(time.delta()).just_finished() {
            let side = (time.elapsed_secs() * 13.0).sin();
            commands.spawn((
                LevelEntity,
                Mesh3d(assets.small_sphere.clone()),
                MeshMaterial3d(assets.slobber.clone()),
                Transform {
                    translation: tf.translation + Vec3::new(side * 0.3, 0.5, side.cos() * 0.3),
                    scale: Vec3::splat(0.14),
                    ..default()
                },
                Particle {
                    velocity: Vec3::new(side * 0.5, 0.5, 0.0),
                    spin: Vec3::ZERO,
                    life: 0.6,
                    max_life: 0.6,
                    gravity: 9.0,
                    shrink: true,
                },
            ));
        }
    }
}

/// Recolours the cat's fur while slimed.
fn slimy_skin(
    assets: Res<GameAssets>,
    cats: Query<(&Children, Has<Slimy>), With<Cat>>,
    mut skins: Query<(&CatSkin, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (children, is_slimy) in &cats {
        for child in children.iter() {
            if let Ok((skin, mut mat)) = skins.get_mut(child) {
                let want = match (is_slimy, skin.stripe) {
                    (true, true) => &assets.cat_slimy_stripe,
                    (true, false) => &assets.cat_slimy,
                    (false, true) => &assets.cat_stripe,
                    (false, false) => &assets.cat_fur,
                };
                if mat.0 != *want {
                    mat.0 = want.clone();
                }
            }
        }
    }
}
