//! Dogs: blocky, floppy-eared, and far too friendly. They patrol their island,
//! chase the cat when it comes close, and lick it to make it slimy. Two swipes
//! send a dog tumbling off the cat tree.

use crate::cat::{Cat, Falling, Hop, slime};
use crate::grid::{Dir, Facing, Grid, GridPos, cell_to_world};
use crate::level::Difficulty;
use crate::world::{GameAssets, HOP_ARC, Particle, Toy, Yarn, island_of};
use crate::{GameSet, LevelEntity, Sfx, in_play};
use bevy::prelude::*;
use std::f32::consts::PI;

pub struct DogPlugin;

impl Plugin for DogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                dog_think.in_set(GameSet::Think).run_if(in_play),
                (animate_licks, animate_hurt, flop_ears).in_set(GameSet::Animate),
            ),
        );
    }
}

#[derive(Component)]
pub struct Dog {
    pub hp: i32,
    pub island: usize,
    pub think: Timer,
    pub chasing: bool,
    /// A random phase so dogs don't all move in lockstep.
    pub seed: f32,
}

#[derive(Component)]
pub struct Tongue;

#[derive(Component)]
pub struct Ear {
    side: f32,
}

/// Fur that flashes when the dog is hit.
#[derive(Component)]
pub struct DogSkin {
    dark: bool,
}

#[derive(Component)]
pub struct Lick {
    t: f32,
    hit_done: bool,
}

#[derive(Component)]
pub struct Hurt {
    pub t: f32,
}

const LICK_TIME: f32 = 0.55;
const DOG_HOP_TIME: f32 = 0.3;

pub fn spawn_dog(commands: &mut Commands, assets: &GameAssets, top: Vec3, cell: IVec2, island: usize, difficulty: &Difficulty) {
    let fur = assets.dog_fur.clone();
    let dark = assets.dog_dark.clone();
    let seed = (cell.x * 7 + cell.y * 13) as f32 * 0.37;
    let facing = Dir::ALL[(cell.x + cell.y).rem_euclid(4) as usize];
    commands
        .spawn((
            LevelEntity,
            Dog {
                hp: 2,
                island,
                think: Timer::from_seconds(difficulty.dog_patrol_interval * (0.5 + seed.rem_euclid(1.0)), TimerMode::Once),
                chasing: false,
                seed,
            },
            GridPos(cell),
            Facing(facing),
            Transform {
                translation: top,
                rotation: Quat::from_rotation_y(facing.yaw()),
                ..default()
            },
            Visibility::default(),
        ))
        .with_children(|p| {
            // Body.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(fur.clone()),
                DogSkin { dark: false },
                Transform {
                    translation: Vec3::new(0.0, 0.42, 0.08),
                    scale: Vec3::new(0.6, 0.5, 0.85),
                    ..default()
                },
            ));
            // Head.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(fur.clone()),
                DogSkin { dark: false },
                Transform {
                    translation: Vec3::new(0.0, 0.78, -0.45),
                    scale: Vec3::new(0.55, 0.5, 0.45),
                    ..default()
                },
            ));
            // Snout.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(dark.clone()),
                DogSkin { dark: true },
                Transform {
                    translation: Vec3::new(0.0, 0.68, -0.75),
                    scale: Vec3::new(0.3, 0.24, 0.24),
                    ..default()
                },
            ));
            // Nose.
            p.spawn((
                Mesh3d(assets.sphere.clone()),
                MeshMaterial3d(assets.black.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.74, -0.88),
                    scale: Vec3::splat(0.12),
                    ..default()
                },
            ));
            // Tongue: hangs out of the mouth and shoots forward when licking.
            p.spawn((
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.tongue.clone()),
                Tongue,
                Transform {
                    translation: Vec3::new(0.0, 0.56, -0.8),
                    scale: Vec3::new(0.14, 0.05, 0.22),
                    ..default()
                },
            ));
            for side in [-1.0, 1.0] {
                // Floppy ears.
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(dark.clone()),
                    DogSkin { dark: true },
                    Ear { side },
                    Transform {
                        translation: Vec3::new(side * 0.32, 0.8, -0.45),
                        scale: Vec3::new(0.1, 0.4, 0.28),
                        ..default()
                    },
                ));
                // Eyes.
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.white.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.15, 0.88, -0.66),
                        scale: Vec3::splat(0.15),
                        ..default()
                    },
                ));
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.black.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.15, 0.88, -0.72),
                        scale: Vec3::splat(0.07),
                        ..default()
                    },
                ));
            }
            // Legs.
            for (x, z) in [(-0.2, -0.25), (0.2, -0.25), (-0.2, 0.35), (0.2, 0.35)] {
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(fur.clone()),
                    DogSkin { dark: false },
                    Transform {
                        translation: Vec3::new(x, 0.14, z),
                        scale: Vec3::new(0.18, 0.28, 0.18),
                        ..default()
                    },
                ));
            }
            // Tail.
            p.spawn((
                Mesh3d(assets.cylinder.clone()),
                MeshMaterial3d(dark.clone()),
                DogSkin { dark: true },
                Transform {
                    translation: Vec3::new(0.0, 0.7, 0.55),
                    rotation: Quat::from_rotation_x(-0.7),
                    scale: Vec3::new(0.1, 0.45, 0.1),
                },
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn dog_think(
    time: Res<Time>,
    mut commands: Commands,
    grid: Res<Grid>,
    difficulty: Res<Difficulty>,
    cat: Query<(Entity, &GridPos, Option<&Hop>), (With<Cat>, Without<Falling>)>,
    mut dogs: Query<(Entity, &GridPos, &Transform, &mut Dog, &mut Facing), (Without<Hop>, Without<Lick>, Without<Particle>)>,
    others: Query<(Entity, &GridPos, Option<&Hop>), Or<(With<Toy>, With<Yarn>, With<Dog>)>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let cat = cat.single().ok();
    let cat_cell = cat.map(|(_, p, _)| p.0);
    let cat_target = cat.and_then(|(_, _, hop)| hop.and_then(|h| h.target));

    for (entity, pos, transform, mut dog, mut facing) in &mut dogs {
        if !dog.think.tick(time.delta()).is_finished() {
            continue;
        }
        let here = pos.0;
        let here_h = grid.height(here).unwrap_or(0);
        let dog_island = dog.island;

        // Is a cell free for this dog to hop onto?
        let occupied = |cell: IVec2| {
            Some(cell) == cat_cell
                || Some(cell) == cat_target
                || others
                    .iter()
                    .any(|(e, p, hop)| e != entity && (p.0 == cell || hop.and_then(|h| h.target) == Some(cell)))
        };
        let can_step = |dir: Dir| -> Option<(IVec2, i32)> {
            let next = here + dir.offset();
            let h = grid.height(next)?;
            if island_of(&grid, next) != Some(dog_island) || (h - here_h).abs() > 1 || occupied(next) {
                return None;
            }
            Some((next, h))
        };

        let mut step: Option<(Dir, IVec2, i32)> = None;
        let mut lick = false;

        if let Some(cat_cell) = cat_cell {
            let delta = cat_cell - here;
            let dist = delta.x.abs() + delta.y.abs();
            let same_island = island_of(&grid, cat_cell) == Some(dog.island);
            if dist == 1 {
                lick = true;
                if let Some(dir) = Dir::from_offset(delta) {
                    facing.0 = dir;
                }
            } else if same_island && dist <= difficulty.dog_chase_range {
                dog.chasing = true;
                let mut prefs = Vec::new();
                if let Some(d) = Dir::toward(delta) {
                    prefs.push(d);
                }
                if let Some(d) = Dir::toward(IVec2::new(if delta.x.abs() >= delta.y.abs() { 0 } else { delta.x }, if delta.x.abs() >= delta.y.abs() { delta.y } else { 0 })) {
                    prefs.push(d);
                }
                for d in prefs {
                    if let Some((cell, h)) = can_step(d) {
                        step = Some((d, cell, h));
                        break;
                    }
                }
            } else {
                dog.chasing = false;
            }
        }

        if lick {
            commands.entity(entity).insert(Lick { t: 0.0, hit_done: false });
            dog.think = Timer::from_seconds(difficulty.dog_chase_interval + 0.4, TimerMode::Once);
            continue;
        }

        if step.is_none() && !dog.chasing {
            // Patrol: wander somewhere on the island, or sit for a moment.
            let roll = (time.elapsed_secs() * 3.1 + dog.seed).sin() * 0.5 + 0.5;
            if roll > 0.3 {
                let start = ((time.elapsed_secs() * 7.7 + dog.seed) * 4.0) as usize;
                for i in 0..4 {
                    let d = Dir::ALL[(start + i) % 4];
                    if let Some((cell, h)) = can_step(d) {
                        step = Some((d, cell, h));
                        break;
                    }
                }
            }
        }

        if let Some((dir, cell, h)) = step {
            facing.0 = dir;
            commands.entity(entity).insert(Hop {
                from: transform.translation,
                to: cell_to_world(cell, h),
                t: 0.0,
                duration: DOG_HOP_TIME,
                arc: HOP_ARC * 0.8,
                target: Some(cell),
            });
        }

        let interval = if dog.chasing {
            difficulty.dog_chase_interval
        } else {
            difficulty.dog_patrol_interval
        };
        dog.think = Timer::from_seconds(interval, TimerMode::Once);
    }
    let _ = &mut sfx;
}

#[allow(clippy::too_many_arguments)]
fn animate_licks(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut dogs: Query<(Entity, &GridPos, &Facing, &Transform, &mut Lick, &Children)>,
    mut tongues: Query<&mut Transform, (With<Tongue>, Without<Lick>)>,
    cat: Query<(Entity, &GridPos), (With<Cat>, Without<Falling>)>,
    mut sfx: MessageWriter<Sfx>,
) {
    for (entity, pos, facing, transform, mut lick, children) in &mut dogs {
        lick.t += time.delta_secs();
        let s = (lick.t / LICK_TIME).min(1.0);
        let reach = (PI * s).sin();
        for child in children.iter() {
            if let Ok(mut tongue) = tongues.get_mut(child) {
                tongue.translation = Vec3::new(0.0, 0.56, -0.8 - reach * 0.6);
                tongue.scale = Vec3::new(0.14, 0.05, 0.22 + reach * 1.0);
            }
        }
        if !lick.hit_done && s >= 0.5 {
            lick.hit_done = true;
            let target = pos.0 + facing.0.offset();
            if let Some((cat_entity, _)) = cat.iter().find(|(_, p)| p.0 == target) {
                slime(&mut commands, cat_entity);
                sfx.write(Sfx::Lick);
                // Slobber splash.
                let at = transform.translation + Vec3::new(facing.0.offset().x as f32, 0.6, facing.0.offset().y as f32) * 0.8;
                for i in 0..6 {
                    let a = i as f32 * 1.05;
                    commands.spawn((
                        LevelEntity,
                        Mesh3d(assets.small_sphere.clone()),
                        MeshMaterial3d(assets.slobber.clone()),
                        Transform {
                            translation: at,
                            scale: Vec3::splat(0.18),
                            ..default()
                        },
                        Particle {
                            velocity: Vec3::new(a.cos() * 1.5, 2.5, a.sin() * 1.5),
                            spin: Vec3::ZERO,
                            life: 0.7,
                            max_life: 0.7,
                            gravity: 9.0,
                            shrink: true,
                        },
                    ));
                }
            }
        }
        if s >= 1.0 {
            commands.entity(entity).remove::<Lick>();
        }
    }
}

fn animate_hurt(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut dogs: Query<(Entity, &mut Hurt, &Children)>,
    mut skins: Query<(&DogSkin, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, mut hurt, children) in &mut dogs {
        hurt.t += time.delta_secs();
        let done = hurt.t >= 0.35;
        let flash = !done && (hurt.t * 30.0).sin() > 0.0;
        for child in children.iter() {
            if let Ok((skin, mut mat)) = skins.get_mut(child) {
                let want = if flash {
                    &assets.dog_hurt
                } else if skin.dark {
                    &assets.dog_dark
                } else {
                    &assets.dog_fur
                };
                if mat.0 != *want {
                    mat.0 = want.clone();
                }
            }
        }
        if done {
            commands.entity(entity).remove::<Hurt>();
        }
    }
}

fn flop_ears(time: Res<Time>, mut ears: Query<(&mut Transform, &Ear)>) {
    let t = time.elapsed_secs();
    for (mut tf, ear) in &mut ears {
        tf.rotation = Quat::from_rotation_z(ear.side * (0.2 + (t * 5.0).sin() * 0.15));
    }
}
