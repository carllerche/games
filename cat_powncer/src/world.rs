//! Building the visible world: meshes and materials, the floating islands,
//! decorations, the camera rig, lighting, and simple particle effects.

use crate::grid::{Axis, FLOOR_Y, Grid, GridPos, LEVEL_HEIGHT, TILE_THICKNESS, TileKind, cell_to_world, level_y};
use crate::level::{self, ToyKind, YarnColor};
use crate::{AppState, CurrentLevel, LevelEntity, LoadLevel, Phase, ScreenshotRequest, Sfx, StartLevel, cat, dog};
use bevy::camera::{RenderTarget, ScalingMode};
use bevy::render::render_resource::TextureFormat;
use bevy::light::light_consts::lux;
use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Grid>()
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb_u8(255, 245, 230),
                brightness: 900.0,
                ..default()
            })
            .add_systems(Startup, (load_assets, spawn_camera_and_lights).chain())
            .add_systems(OnEnter(AppState::Playing), start_from_level_one)
            .add_systems(OnExit(AppState::Playing), despawn_level)
            .add_systems(
                Update,
                (
                    build_level.run_if(on_message::<LoadLevel>),
                    follow_camera,
                    spin_and_bob,
                    particles,
                    squash_stretch,
                ),
            );
    }
}

/// Shared mesh and material handles. Every model in the game is assembled
/// from these few primitives.
#[derive(Resource)]
pub struct GameAssets {
    pub tile: Handle<Mesh>,
    pub plank_x: Handle<Mesh>,
    pub plank_z: Handle<Mesh>,
    pub cube: Handle<Mesh>,
    pub sphere: Handle<Mesh>,
    pub small_sphere: Handle<Mesh>,
    pub cylinder: Handle<Mesh>,
    pub cone: Handle<Mesh>,
    pub torus: Handle<Mesh>,
    pub capsule: Handle<Mesh>,
    pub floor: Handle<Mesh>,

    pub carpets: Vec<Handle<StandardMaterial>>,
    pub plank: Handle<StandardMaterial>,
    pub post: Handle<StandardMaterial>,
    pub post_dark: Handle<StandardMaterial>,
    pub floor_mat: Handle<StandardMaterial>,
    pub start_pad: Handle<StandardMaterial>,

    pub cat_fur: Handle<StandardMaterial>,
    pub cat_stripe: Handle<StandardMaterial>,
    pub cat_belly: Handle<StandardMaterial>,
    pub cat_slimy: Handle<StandardMaterial>,
    pub cat_slimy_stripe: Handle<StandardMaterial>,
    pub black: Handle<StandardMaterial>,
    pub white: Handle<StandardMaterial>,
    pub pink: Handle<StandardMaterial>,

    pub dog_fur: Handle<StandardMaterial>,
    pub dog_dark: Handle<StandardMaterial>,
    pub dog_hurt: Handle<StandardMaterial>,
    pub tongue: Handle<StandardMaterial>,
    pub slobber: Handle<StandardMaterial>,

    pub toy_gray: Handle<StandardMaterial>,
    pub fish: Handle<StandardMaterial>,
    pub fish_fin: Handle<StandardMaterial>,
    pub feather_stem: Handle<StandardMaterial>,
    pub feather_tip: Handle<StandardMaterial>,

    pub yarn: HashMap<YarnColor, Handle<StandardMaterial>>,
    pub poof: Handle<StandardMaterial>,
    pub sparkle: Handle<StandardMaterial>,
}

fn flat(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    })
}

fn glowing(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 0.25,
        perceptual_roughness: 0.7,
        ..default()
    })
}

fn load_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let carpets = [
        Color::srgb_u8(232, 200, 150),
        Color::srgb_u8(180, 210, 160),
        Color::srgb_u8(200, 180, 220),
        Color::srgb_u8(240, 190, 170),
        Color::srgb_u8(170, 205, 225),
        Color::srgb_u8(230, 215, 170),
    ]
    .into_iter()
    .map(|c| flat(&mut materials, c))
    .collect();

    let yarn = [
        YarnColor::Purple,
        YarnColor::Red,
        YarnColor::Blue,
        YarnColor::Green,
        YarnColor::Yellow,
        YarnColor::Orange,
    ]
    .into_iter()
    .map(|y| (y, glowing(&mut materials, y.color())))
    .collect();

    commands.insert_resource(GameAssets {
        tile: meshes.add(Cuboid::new(1.0, TILE_THICKNESS, 1.0)),
        plank_x: meshes.add(Cuboid::new(1.0, 0.16, 0.7)),
        plank_z: meshes.add(Cuboid::new(0.7, 0.16, 1.0)),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        sphere: meshes.add(Sphere::new(0.5).mesh().ico(3).unwrap()),
        small_sphere: meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap()),
        cylinder: meshes.add(Cylinder::new(0.5, 1.0)),
        cone: meshes.add(Cone::new(0.5, 1.0)),
        torus: meshes.add(Torus::new(0.28, 0.36)),
        capsule: meshes.add(Capsule3d::new(0.5, 1.0)),
        floor: meshes.add(Plane3d::default().mesh().size(400.0, 400.0)),

        carpets,
        plank: flat(&mut materials, Color::srgb_u8(180, 120, 70)),
        post: flat(&mut materials, Color::srgb_u8(205, 170, 110)),
        post_dark: flat(&mut materials, Color::srgb_u8(170, 135, 85)),
        floor_mat: flat(&mut materials, Color::srgb_u8(214, 232, 236)),
        start_pad: flat(&mut materials, Color::srgb_u8(255, 235, 120)),

        cat_fur: flat(&mut materials, Color::srgb_u8(245, 160, 60)),
        cat_stripe: flat(&mut materials, Color::srgb_u8(200, 110, 40)),
        cat_belly: flat(&mut materials, Color::srgb_u8(255, 235, 200)),
        cat_slimy: flat(&mut materials, Color::srgb_u8(150, 200, 90)),
        cat_slimy_stripe: flat(&mut materials, Color::srgb_u8(110, 160, 60)),
        black: flat(&mut materials, Color::srgb_u8(30, 30, 35)),
        white: flat(&mut materials, Color::WHITE),
        pink: flat(&mut materials, Color::srgb_u8(240, 130, 160)),

        dog_fur: flat(&mut materials, Color::srgb_u8(150, 100, 60)),
        dog_dark: flat(&mut materials, Color::srgb_u8(100, 65, 40)),
        dog_hurt: glowing(&mut materials, Color::srgb_u8(255, 120, 120)),
        tongue: flat(&mut materials, Color::srgb_u8(240, 100, 130)),
        slobber: materials.add(StandardMaterial {
            base_color: Color::srgba_u8(170, 230, 120, 200),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.3,
            ..default()
        }),

        toy_gray: flat(&mut materials, Color::srgb_u8(150, 150, 160)),
        fish: flat(&mut materials, Color::srgb_u8(70, 200, 190)),
        fish_fin: flat(&mut materials, Color::srgb_u8(40, 150, 150)),
        feather_stem: flat(&mut materials, Color::srgb_u8(240, 240, 240)),
        feather_tip: flat(&mut materials, Color::srgb_u8(80, 200, 220)),

        yarn,
        poof: flat(&mut materials, Color::srgb_u8(250, 250, 250)),
        sparkle: glowing(&mut materials, Color::srgb_u8(255, 240, 150)),
    });
}

// ---------------------------------------------------------------------------
// Camera and lights
// ---------------------------------------------------------------------------

/// Where the camera is looking; it eases toward the cat every frame.
#[derive(Component)]
pub struct CameraRig {
    pub target: Vec3,
}

const CAMERA_YAW: f32 = 0.55;
const CAMERA_PITCH: f32 = 0.82;

fn camera_offset() -> Vec3 {
    Vec3::new(
        CAMERA_YAW.sin() * CAMERA_PITCH.cos(),
        CAMERA_PITCH.sin(),
        CAMERA_YAW.cos() * CAMERA_PITCH.cos(),
    ) * 60.0
}

fn spawn_camera_and_lights(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut images: ResMut<Assets<Image>>,
    screenshot: Option<ResMut<ScreenshotRequest>>,
) {
    // In screenshot mode the camera draws into an offscreen image we can read back.
    let mut camera = commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 13.0,
            },
            far: 400.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(camera_offset()).looking_at(Vec3::ZERO, Vec3::Y),
        CameraRig { target: Vec3::ZERO },
        IsDefaultUiCamera,
    ));
    if let Some(mut req) = screenshot {
        let image = images.add(Image::new_target_texture(
            1280,
            800,
            TextureFormat::Rgba8Unorm,
            Some(TextureFormat::Rgba8UnormSrgb),
        ));
        camera.insert(RenderTarget::Image(image.clone().into()));
        req.target = Some(image);
    }

    commands.spawn((
        DirectionalLight {
            illuminance: lux::OVERCAST_DAY * 1.6,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-6.0, 14.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // The living-room floor far below the cat tree.
    commands.spawn((
        Mesh3d(assets.floor.clone()),
        MeshMaterial3d(assets.floor_mat.clone()),
        Transform::from_xyz(0.0, FLOOR_Y, 0.0),
    ));
}

fn follow_camera(
    time: Res<Time>,
    cat: Query<&Transform, (With<cat::Cat>, Without<CameraRig>)>,
    mut rig: Single<(&mut Transform, &mut CameraRig)>,
) {
    let (transform, rig) = &mut *rig;
    if let Ok(cat_tf) = cat.single() {
        let mut goal = cat_tf.translation;
        goal.y = goal.y.max(0.0);
        let k = 1.0 - (-time.delta_secs() * 6.0).exp();
        rig.target = rig.target.lerp(goal, k);
    }
    *transform.as_mut() = Transform::from_translation(rig.target + camera_offset()).looking_at(rig.target, Vec3::Y);
}

// ---------------------------------------------------------------------------
// Level construction
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct Toy;

#[derive(Component)]
pub struct Yarn(pub YarnColor);

fn start_from_level_one(mut level: ResMut<CurrentLevel>, mut start: ResMut<StartLevel>, mut load: MessageWriter<LoadLevel>) {
    level.0 = start.0.take().unwrap_or(1);
    load.write(LoadLevel);
}

fn despawn_level(mut commands: Commands, entities: Query<Entity, With<LevelEntity>>) {
    for e in &entities {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn build_level(
    mut commands: Commands,
    assets: Res<GameAssets>,
    level: Res<CurrentLevel>,
    existing: Query<Entity, With<LevelEntity>>,
    mut grid: ResMut<Grid>,
    mut phase: ResMut<Phase>,
    mut rig: Single<(&mut Transform, &mut CameraRig)>,
    mut sfx: MessageWriter<Sfx>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    let data = level::generate(level.0);
    let mut rng = crate::rng::Rng::new(level.0 as u64 * 31 + 7);

    // Tiles.
    for (&cell, tile) in &data.grid.tiles {
        let top = cell_to_world(cell, tile.height);
        match tile.kind {
            TileKind::Island(i) => {
                let carpet = assets.carpets[i % assets.carpets.len()].clone();
                let material = if cell == data.grid.start {
                    assets.start_pad.clone()
                } else {
                    carpet
                };
                commands.spawn((
                    LevelEntity,
                    Mesh3d(assets.tile.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(top - Vec3::Y * TILE_THICKNESS * 0.5),
                ));
            }
            TileKind::Bridge(axis) => {
                let mesh = match axis {
                    Axis::X => assets.plank_x.clone(),
                    Axis::Z => assets.plank_z.clone(),
                };
                commands.spawn((
                    LevelEntity,
                    Mesh3d(mesh),
                    MeshMaterial3d(assets.plank.clone()),
                    Transform::from_translation(top - Vec3::Y * 0.08),
                ));
            }
        }
    }

    // A sisal-wrapped post under each island, reaching down to the floor.
    for island in &data.islands {
        let top_y = level_y(island.height) - TILE_THICKNESS;
        let bottom = FLOOR_Y;
        let h = top_y - bottom;
        let radius = 0.55 + island.cells.len() as f32 * 0.02;
        commands.spawn((
            LevelEntity,
            Mesh3d(assets.cylinder.clone()),
            MeshMaterial3d(assets.post.clone()),
            Transform {
                translation: Vec3::new(island.center.x, bottom + h * 0.5, island.center.y),
                scale: Vec3::new(radius * 2.0, h, radius * 2.0),
                ..default()
            },
        ));
        // Rope rings so the post reads as a scratching post.
        let mut y = top_y - 0.8;
        while y > bottom + 1.0 {
            commands.spawn((
                LevelEntity,
                Mesh3d(assets.torus.clone()),
                MeshMaterial3d(assets.post_dark.clone()),
                Transform {
                    translation: Vec3::new(island.center.x, y, island.center.y),
                    scale: Vec3::splat(radius * 2.6),
                    rotation: Quat::from_rotation_x(FRAC_PI_2),
                },
            ));
            y -= 1.6;
        }
        // Island skirt: a wooden base under the carpet.
        let (min, max) = island
            .cells
            .iter()
            .fold((IVec2::MAX, IVec2::MIN), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
        let size = (max - min).as_vec2() + Vec2::ONE;
        let center = (max + min).as_vec2() * 0.5;
        commands.spawn((
            LevelEntity,
            Mesh3d(assets.cube.clone()),
            MeshMaterial3d(assets.plank.clone()),
            Transform {
                translation: Vec3::new(center.x, level_y(island.height) - TILE_THICKNESS - 0.2, center.y),
                scale: Vec3::new(size.x - 0.2, 0.4, size.y - 0.2),
                ..default()
            },
        ));
    }

    // Toys.
    for &(cell, kind) in &data.toys {
        let top = data.grid.top_of(cell).unwrap();
        spawn_toy(&mut commands, &assets, top, cell, kind, rng.f32() * TAU);
    }

    // Yarn balls.
    for &(cell, color) in &data.yarn {
        let top = data.grid.top_of(cell).unwrap();
        commands
            .spawn((
                LevelEntity,
                Yarn(color),
                GridPos(cell),
                Transform::from_translation(top + Vec3::Y * 0.45),
                Visibility::default(),
                Spin(1.4),
                Bob {
                    base_y: top.y + 0.45,
                    amplitude: 0.08,
                    speed: 2.5,
                    phase: rng.f32() * TAU,
                },
            ))
            .with_children(|p| {
                let mat = assets.yarn[&color].clone();
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_scale(Vec3::splat(0.7)),
                ));
                for (i, rot) in [
                    Quat::from_rotation_x(0.6),
                    Quat::from_rotation_z(1.1) * Quat::from_rotation_x(-0.4),
                    Quat::from_rotation_y(1.0) * Quat::from_rotation_x(1.3),
                ]
                .into_iter()
                .enumerate()
                {
                    p.spawn((
                        Mesh3d(assets.torus.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform {
                            rotation: rot,
                            scale: Vec3::splat(1.0 + i as f32 * 0.04),
                            ..default()
                        },
                    ));
                }
            });
    }

    // Dogs.
    for &(cell, island) in &data.dogs {
        let top = data.grid.top_of(cell).unwrap();
        dog::spawn_dog(&mut commands, &assets, top, cell, island, &data.difficulty);
    }

    // The cat.
    let start_top = data.grid.top_of(data.grid.start).unwrap();
    cat::spawn_cat(&mut commands, &assets, start_top, data.grid.start);

    // Snap the camera onto the start.
    let (transform, rig) = &mut *rig;
    rig.target = start_top;
    *transform.as_mut() = Transform::from_translation(rig.target + camera_offset()).looking_at(rig.target, Vec3::Y);

    commands.insert_resource(data.difficulty);
    *grid = data.grid;
    *phase = Phase::Intro(Timer::from_seconds(1.4, TimerMode::Once));
    sfx.write(Sfx::LevelStart);
}

fn spawn_toy(commands: &mut Commands, assets: &GameAssets, top: Vec3, cell: IVec2, kind: ToyKind, yaw: f32) {
    let mut root = commands.spawn((
        LevelEntity,
        Toy,
        GridPos(cell),
        Transform {
            translation: top,
            rotation: Quat::from_rotation_y(yaw),
            ..default()
        },
        Visibility::default(),
    ));
    root.with_children(|p| match kind {
        ToyKind::Mouse => {
            p.spawn((
                Mesh3d(assets.capsule.clone()),
                MeshMaterial3d(assets.toy_gray.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.2, 0.0),
                    rotation: Quat::from_rotation_x(FRAC_PI_2),
                    scale: Vec3::new(0.38, 0.3, 0.38),
                },
            ));
            for side in [-1.0, 1.0] {
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.pink.clone()),
                    Transform {
                        translation: Vec3::new(side * 0.14, 0.32, -0.2),
                        scale: Vec3::splat(0.14),
                        ..default()
                    },
                ));
            }
            p.spawn((
                Mesh3d(assets.cylinder.clone()),
                MeshMaterial3d(assets.pink.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.12, 0.5),
                    rotation: Quat::from_rotation_x(FRAC_PI_2),
                    scale: Vec3::new(0.06, 0.5, 0.06),
                },
            ));
        }
        ToyKind::Fish => {
            // A flat catnip fish: body, tail fin, and an eye.
            p.spawn((
                Mesh3d(assets.capsule.clone()),
                MeshMaterial3d(assets.fish.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.16, 0.0),
                    rotation: Quat::from_rotation_x(FRAC_PI_2),
                    scale: Vec3::new(0.42, 0.34, 0.18),
                },
            ));
            p.spawn((
                Mesh3d(assets.cone.clone()),
                MeshMaterial3d(assets.fish_fin.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.16, 0.5),
                    rotation: Quat::from_rotation_x(-FRAC_PI_2),
                    scale: Vec3::new(0.34, 0.3, 0.12),
                },
            ));
            p.spawn((
                Mesh3d(assets.sphere.clone()),
                MeshMaterial3d(assets.white.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.28, -0.25),
                    scale: Vec3::splat(0.12),
                    ..default()
                },
            ));
            p.spawn((
                Mesh3d(assets.sphere.clone()),
                MeshMaterial3d(assets.black.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.32, -0.27),
                    scale: Vec3::splat(0.06),
                    ..default()
                },
            ));
        }
        ToyKind::Feather => {
            p.spawn((
                Mesh3d(assets.cylinder.clone()),
                MeshMaterial3d(assets.feather_stem.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.35, 0.0),
                    rotation: Quat::from_rotation_z(0.35),
                    scale: Vec3::new(0.06, 0.7, 0.06),
                },
            ));
            p.spawn((
                Mesh3d(assets.capsule.clone()),
                MeshMaterial3d(assets.feather_tip.clone()),
                Transform {
                    translation: Vec3::new(-0.2, 0.75, 0.0),
                    rotation: Quat::from_rotation_z(0.35),
                    scale: Vec3::new(0.32, 0.35, 0.1),
                },
            ));
        }
    });
}

// ---------------------------------------------------------------------------
// Small generic animations
// ---------------------------------------------------------------------------

/// Rotate around Y at `radians / second`.
#[derive(Component)]
pub struct Spin(pub f32);

#[derive(Component)]
pub struct Bob {
    pub base_y: f32,
    pub amplitude: f32,
    pub speed: f32,
    pub phase: f32,
}

fn spin_and_bob(
    time: Res<Time>,
    mut spinners: Query<(&mut Transform, Option<&Spin>, Option<&Bob>), Or<(With<Spin>, With<Bob>)>>,
) {
    let t = time.elapsed_secs();
    for (mut tf, spin, bob) in &mut spinners {
        if let Some(spin) = spin {
            tf.rotate_y(spin.0 * time.delta_secs());
        }
        if let Some(bob) = bob {
            tf.translation.y = bob.base_y + (t * bob.speed + bob.phase).sin() * bob.amplitude;
        }
    }
}

/// A cartoon squash on landing, springing back to normal scale.
#[derive(Component)]
pub struct Squash {
    pub t: f32,
}

fn squash_stretch(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut Transform, &mut Squash)>) {
    for (e, mut tf, mut s) in &mut q {
        s.t += time.delta_secs();
        let d = 0.28;
        if s.t >= d {
            tf.scale = Vec3::ONE;
            commands.entity(e).remove::<Squash>();
            continue;
        }
        let k = (s.t / d * PI).sin();
        tf.scale = Vec3::new(1.0 + 0.25 * k, 1.0 - 0.3 * k, 1.0 + 0.25 * k);
    }
}

/// A flying blob: a poof cloud, a slobber drop, a knocked-off toy, a fleeing dog...
#[derive(Component)]
pub struct Particle {
    pub velocity: Vec3,
    pub spin: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub gravity: f32,
    pub shrink: bool,
}

fn particles(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut Transform, &mut Particle)>) {
    let dt = time.delta_secs();
    for (e, mut tf, mut p) in &mut q {
        p.life -= dt;
        p.velocity.y -= p.gravity * dt;
        let v = p.velocity;
        tf.translation += v * dt;
        let spin = p.spin;
        tf.rotate(Quat::from_euler(EulerRot::XYZ, spin.x * dt, spin.y * dt, spin.z * dt));
        if p.shrink {
            let k = (p.life / p.max_life).clamp(0.0, 1.0);
            tf.scale = Vec3::splat(k);
        }
        if p.life <= 0.0 || tf.translation.y < FLOOR_Y - 2.0 {
            commands.entity(e).despawn();
        }
    }
}

/// A puff of little clouds.
pub fn spawn_poof(commands: &mut Commands, assets: &GameAssets, at: Vec3, count: usize, material: Handle<StandardMaterial>, speed: f32) {
    for i in 0..count {
        let a = i as f32 / count as f32 * TAU + at.x * 7.0;
        let dir = Vec3::new(a.cos(), 0.8 + (a * 3.0).sin() * 0.3, a.sin());
        commands.spawn((
            LevelEntity,
            Mesh3d(assets.small_sphere.clone()),
            MeshMaterial3d(material.clone()),
            Transform {
                translation: at + dir * 0.15,
                scale: Vec3::splat(0.35),
                ..default()
            },
            Particle {
                velocity: dir * speed,
                spin: Vec3::ZERO,
                life: 0.5,
                max_life: 0.5,
                gravity: 2.0,
                shrink: true,
            },
        ));
    }
}

/// Turns an actor into a tumbling projectile (a swiped toy, a defeated dog).
pub fn fling(commands: &mut Commands, entity: Entity, direction: Vec3, power: f32) {
    commands
        .entity(entity)
        .remove::<(GridPos, Spin, Bob)>()
        .insert(Particle {
            velocity: direction * power + Vec3::Y * power * 1.1,
            spin: Vec3::new(direction.z * 8.0, 3.0, -direction.x * 8.0),
            life: 4.0,
            max_life: 4.0,
            gravity: 16.0,
            shrink: false,
        });
}

/// Which island a cell belongs to, if it is not a bridge.
pub fn island_of(grid: &Grid, cell: IVec2) -> Option<usize> {
    grid.tile(cell).and_then(|t| t.island())
}

/// Height of one level in world units, re-exported for other modules' animation math.
pub const HOP_ARC: f32 = LEVEL_HEIGHT * 0.5;
