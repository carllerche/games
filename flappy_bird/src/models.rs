//! Procedurally built models.
//!
//! Every model is assembled from four unit-size primitives (sphere, cube,
//! cylinder, cone) scaled and positioned by `Transform`. Each part gets a
//! black "inverted hull" outline: a copy of the mesh, slightly enlarged, drawn
//! with front faces culled so only a thin rim shows around the silhouette.
//! Together with the cel-shaded material this gives the flat, chunky,
//! hand-inked look of The Wind Waker.

use bevy::{platform::collections::HashMap, prelude::*, render::render_resource::Face};
use flappy_core::{palette, *};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::toon::ToonMaterial;

/// Outline thickness in world units (each side).
const OUTLINE: f32 = 1.4;

/// How deep (along Z) each ground tile is, and where its center sits.
pub const GROUND_DEPTH: f32 = 420.0;
pub const GROUND_Z: f32 = -110.0;
/// The far edge of the ground, which is where the camera sees widest.
pub const GROUND_FAR_Z: f32 = GROUND_Z - GROUND_DEPTH / 2.0;

pub const SEA: Color = Color::srgb(0.16, 0.55, 0.85);
pub const SAND: Color = Color::srgb(0.93, 0.87, 0.62);
pub const ROCK: Color = Color::srgb(0.58, 0.60, 0.62);
pub const TRUNK: Color = Color::srgb(0.55, 0.38, 0.22);
pub const LEAF: Color = Color::srgb(0.22, 0.62, 0.30);
pub const CLOUD: Color = Color::srgb(0.98, 0.98, 1.0);

#[derive(Clone, Copy)]
pub enum Shape {
    Sphere,
    Cube,
    Cylinder,
    Cone,
}

/// A flapping wing pivot. `sign` flips the rotation so both wings move up
/// together.
#[derive(Component)]
pub struct Wing {
    pub sign: f32,
}

/// Shared meshes and materials.
#[derive(Resource)]
pub struct ModelKit {
    sphere: Handle<Mesh>,
    cube: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    cone: Handle<Mesh>,
    outline: Handle<StandardMaterial>,
    materials: HashMap<[u32; 4], Handle<ToonMaterial>>,
}

impl ModelKit {
    pub fn new(meshes: &mut Assets<Mesh>, standard: &mut Assets<StandardMaterial>) -> Self {
        Self {
            // All unit-sized so a part's Transform scale is its extent.
            sphere: meshes.add(Sphere::new(0.5).mesh().ico(3).unwrap()),
            cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            cylinder: meshes.add(Cylinder::new(0.5, 1.0).mesh().resolution(24)),
            cone: meshes.add(
                Cone {
                    radius: 0.5,
                    height: 1.0,
                }
                .mesh()
                .resolution(16),
            ),
            outline: standard.add(StandardMaterial {
                base_color: Color::BLACK,
                unlit: true,
                cull_mode: Some(Face::Front),
                ..default()
            }),
            materials: HashMap::new(),
        }
    }

    pub fn mesh(&self, shape: Shape) -> Handle<Mesh> {
        match shape {
            Shape::Sphere => self.sphere.clone(),
            Shape::Cube => self.cube.clone(),
            Shape::Cylinder => self.cylinder.clone(),
            Shape::Cone => self.cone.clone(),
        }
    }

    /// One material per distinct color, so Bevy can batch draws.
    pub fn material(
        &mut self,
        toon: &mut Assets<ToonMaterial>,
        color: Color,
    ) -> Handle<ToonMaterial> {
        let key = color.to_linear().to_f32_array().map(f32::to_bits);
        self.materials
            .entry(key)
            .or_insert_with(|| toon.add(ToonMaterial::new(color)))
            .clone()
    }
}

/// Bundles the handles needed to spawn model parts.
pub struct Builder<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub kit: &'a mut ModelKit,
    pub toon: &'a mut Assets<ToonMaterial>,
}

impl Builder<'_, '_, '_> {
    /// Spawn one primitive as a child of `parent`, optionally outlined.
    pub fn part(
        &mut self,
        parent: Entity,
        shape: Shape,
        color: Color,
        transform: Transform,
        outline: bool,
    ) -> Entity {
        let mesh = self.kit.mesh(shape);
        let material = self.kit.material(self.toon, color);
        let entity = self
            .commands
            .spawn((
                ChildOf(parent),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                transform,
            ))
            .id();
        if outline {
            self.outline(entity, mesh, transform.scale);
        }
        entity
    }

    /// Add an inverted-hull outline as a child of an already spawned part.
    fn outline(&mut self, part: Entity, mesh: Handle<Mesh>, scale: Vec3) {
        // Grow each axis by the outline thickness, in the part's own units,
        // so thin parts don't get fatter lines than thick ones.
        let grow = (scale + Vec3::splat(2.0 * OUTLINE)) / scale;
        self.commands.spawn((
            ChildOf(part),
            Mesh3d(mesh),
            MeshMaterial3d(self.kit.outline.clone()),
            Transform::from_scale(grow),
        ));
    }

    // -----------------------------------------------------------------------
    // Models
    // -----------------------------------------------------------------------

    /// A round cartoon bird facing +X, with big eyes, a tuft, and wings that
    /// stick out sideways (along Z) so they read from the camera angle.
    pub fn bird(&mut self, root: Entity) {
        let at = |x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32| {
            Transform::from_xyz(x, y, z).with_scale(Vec3::new(sx, sy, sz))
        };

        // Body and head.
        self.part(
            root,
            Shape::Sphere,
            palette::BODY,
            at(0.0, 0.0, 0.0, 34.0, 24.0, 24.0),
            true,
        );
        self.part(
            root,
            Shape::Sphere,
            palette::BODY,
            at(12.0, 8.0, 0.0, 21.0, 21.0, 21.0),
            true,
        );

        // Feather tuft on top of the head.
        self.part(
            root,
            Shape::Cone,
            palette::WING,
            at(10.0, 19.0, 0.0, 5.0, 8.0, 5.0).with_rotation(Quat::from_rotation_z(0.35)),
            true,
        );

        // Beak: a cone rotated to point along +X.
        self.part(
            root,
            Shape::Cone,
            palette::BEAK,
            at(24.0, 6.0, 0.0, 7.0, 11.0, 7.0).with_rotation(Quat::from_rotation_z(-FRAC_PI_2)),
            true,
        );

        // Eyes on both sides of the head.
        for side in [1.0, -1.0] {
            self.part(
                root,
                Shape::Sphere,
                palette::EYE,
                at(16.0, 9.5, 8.5 * side, 5.5, 7.5, 3.5),
                true,
            );
            self.part(
                root,
                Shape::Sphere,
                palette::PUPIL,
                at(17.8, 9.8, 10.0 * side, 2.6, 3.4, 1.8),
                false,
            );
        }

        // Wings: a pivot at the body's side, with the wing mesh hanging off it.
        for side in [1.0, -1.0] {
            let pivot = self
                .commands
                .spawn((
                    ChildOf(root),
                    Wing { sign: -side },
                    Transform::from_xyz(-3.0, 3.0, 10.0 * side),
                    Visibility::default(),
                ))
                .id();
            self.part(
                pivot,
                Shape::Cube,
                palette::WING,
                at(-2.0, 0.0, 11.0 * side, 18.0, 3.0, 22.0)
                    .with_rotation(Quat::from_rotation_y(0.25 * side)),
                true,
            );
        }

        // Tail feathers.
        self.part(
            root,
            Shape::Cube,
            palette::WING,
            at(-19.0, 3.0, 0.0, 10.0, 3.0, 9.0).with_rotation(Quat::from_rotation_z(0.35)),
            true,
        );
    }

    /// Two green pipes with lips, as children of a pipe pair entity.
    pub fn pipes(&mut self, pair: Entity, g: &PipeGeometry) {
        // Pipes extend past the ceiling and into the ground so their ends
        // never show from the slightly elevated camera.
        const OVERSHOOT: f32 = 200.0;
        let rim_size = Vec3::new(PIPE_WIDTH + 10.0, 24.0, PIPE_WIDTH + 10.0);

        let top_height = g.top_height + OVERSHOOT;
        self.part(
            pair,
            Shape::Cylinder,
            palette::PIPE,
            Transform::from_xyz(0.0, g.top_y + OVERSHOOT / 2.0, 0.0)
                .with_scale(Vec3::new(PIPE_WIDTH, top_height, PIPE_WIDTH)),
            true,
        );
        self.part(
            pair,
            Shape::Cylinder,
            palette::PIPE_RIM,
            Transform::from_xyz(0.0, g.top_y - g.top_height / 2.0 + 12.0, 0.0).with_scale(rim_size),
            true,
        );

        let bottom_height = g.bottom_height + OVERSHOOT;
        self.part(
            pair,
            Shape::Cylinder,
            palette::PIPE,
            Transform::from_xyz(0.0, g.bottom_y - OVERSHOOT / 2.0, 0.0).with_scale(Vec3::new(
                PIPE_WIDTH,
                bottom_height,
                PIPE_WIDTH,
            )),
            true,
        );
        self.part(
            pair,
            Shape::Cylinder,
            palette::PIPE_RIM,
            Transform::from_xyz(0.0, g.bottom_y + g.bottom_height / 2.0 - 12.0, 0.0)
                .with_scale(rim_size),
            true,
        );
    }

    /// A slab of grassy island with tufts and rocks, one window wide.
    /// The parent is centered on the ground's vertical middle, like the 2D tile.
    pub fn ground_tile(&mut self, root: Entity, rng: &mut impl Rng) {
        const DEPTH: f32 = GROUND_DEPTH;
        const Z: f32 = GROUND_Z;
        const GRASS_THICKNESS: f32 = 16.0;
        let top = GROUND_HEIGHT / 2.0;

        self.part(
            root,
            Shape::Cube,
            palette::GRASS,
            Transform::from_xyz(0.0, top - GRASS_THICKNESS / 2.0, Z).with_scale(Vec3::new(
                WINDOW_WIDTH,
                GRASS_THICKNESS,
                DEPTH,
            )),
            false,
        );
        self.part(
            root,
            Shape::Cube,
            SAND,
            Transform::from_xyz(
                0.0,
                top - GRASS_THICKNESS - (GROUND_HEIGHT - GRASS_THICKNESS) / 2.0,
                Z,
            )
            .with_scale(Vec3::new(
                WINDOW_WIDTH,
                GROUND_HEIGHT - GRASS_THICKNESS,
                DEPTH,
            )),
            false,
        );

        for _ in 0..16 {
            let x = rng.random_range(-WINDOW_WIDTH / 2.0 + 10.0..WINDOW_WIDTH / 2.0 - 10.0);
            let z = rng.random_range(Z - DEPTH / 2.0 + 20.0..Z + DEPTH / 2.0 - 20.0);
            let h = rng.random_range(7.0..12.0);
            self.part(
                root,
                Shape::Cone,
                palette::GRASS_DARK,
                Transform::from_xyz(x, top + h / 2.0 - 1.0, z).with_scale(Vec3::new(7.0, h, 7.0)),
                false,
            );
        }

        for _ in 0..3 {
            let x = rng.random_range(-WINDOW_WIDTH / 2.0 + 20.0..WINDOW_WIDTH / 2.0 - 20.0);
            let z = rng.random_range(Z - DEPTH / 2.0 + 40.0..Z - 60.0);
            let w = rng.random_range(12.0..22.0);
            self.part(
                root,
                Shape::Sphere,
                ROCK,
                Transform::from_xyz(x, top + w * 0.25, z)
                    .with_scale(Vec3::new(w, w * 0.7, w * 0.85))
                    .with_rotation(Quat::from_rotation_y(rng.random_range(0.0..TAU))),
                true,
            );
        }
    }

    /// A puffy cluster of flattened spheres.
    pub fn cloud(&mut self, root: Entity, rng: &mut impl Rng) {
        let puffs = rng.random_range(3..=5);
        let mut x = -(puffs as f32) * 14.0;
        for i in 0..puffs {
            let w = rng.random_range(40.0..70.0);
            let lift = if i == 0 || i == puffs - 1 { -6.0 } else { 6.0 };
            self.part(
                root,
                Shape::Sphere,
                CLOUD,
                Transform::from_xyz(x, lift, rng.random_range(-10.0..10.0)).with_scale(Vec3::new(
                    w,
                    w * 0.6,
                    w * 0.7,
                )),
                true,
            );
            x += w * 0.55;
        }
    }

    /// A small distant island: a green mound, a peak, and a palm tree.
    pub fn island(&mut self, root: Entity, rng: &mut impl Rng) {
        let w = rng.random_range(160.0..320.0);
        self.part(
            root,
            Shape::Sphere,
            SAND,
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(w, w * 0.25, w * 0.8)),
            true,
        );
        self.part(
            root,
            Shape::Sphere,
            LEAF,
            Transform::from_xyz(-w * 0.1, w * 0.05, 0.0).with_scale(Vec3::new(
                w * 0.7,
                w * 0.35,
                w * 0.6,
            )),
            true,
        );
        let peak = w * rng.random_range(0.4..0.8);
        self.part(
            root,
            Shape::Cone,
            ROCK,
            Transform::from_xyz(-w * 0.15, peak / 2.0 + w * 0.1, 0.0).with_scale(Vec3::new(
                w * 0.45,
                peak,
                w * 0.4,
            )),
            true,
        );

        let trunk_h = w * 0.35;
        let tx = w * 0.3;
        self.part(
            root,
            Shape::Cylinder,
            TRUNK,
            Transform::from_xyz(tx, trunk_h / 2.0 + w * 0.05, w * 0.1)
                .with_scale(Vec3::new(w * 0.05, trunk_h, w * 0.05))
                .with_rotation(Quat::from_rotation_z(-0.15)),
            true,
        );
        self.part(
            root,
            Shape::Sphere,
            palette::GRASS,
            Transform::from_xyz(tx + trunk_h * 0.15, trunk_h + w * 0.05, w * 0.1)
                .with_scale(Vec3::new(w * 0.28, w * 0.12, w * 0.28)),
            true,
        );
    }

    /// The ocean: one huge flat slab far behind the island.
    pub fn sea(&mut self, root: Entity) {
        self.part(
            root,
            Shape::Cube,
            SEA,
            Transform::from_xyz(0.0, GROUND_TOP - GROUND_HEIGHT - 20.0, -1600.0)
                .with_scale(Vec3::new(8000.0, 10.0, 3000.0)),
            false,
        );
    }

    /// Turn a core-spawned debris entity into a cube of the right color.
    pub fn debris(&mut self, entity: Entity, color: Color) {
        let mesh = self.kit.mesh(Shape::Cube);
        let material = self.kit.material(self.toon, color);
        self.commands
            .entity(entity)
            .insert((Mesh3d(mesh.clone()), MeshMaterial3d(material)));
        self.commands.spawn((
            ChildOf(entity),
            Mesh3d(mesh),
            MeshMaterial3d(self.kit.outline.clone()),
            Transform::from_scale(Vec3::splat(1.3)),
        ));
    }
}
