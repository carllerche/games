//! A darkness overlay drawn over the low-resolution canvas, lit by point
//! lights (the hero, torches, the exit, magic). See `shaders/darkness.wgsl`.

use crate::game::{CANVAS_H, CANVAS_W, CurrentFloor, GameCamera, GameState, Light, layer};
use bevy::{
    asset::embedded_asset,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

const MAX_LIGHTS: usize = 48;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct DarknessMaterial {
    #[uniform(0)]
    params: Vec4,
    #[uniform(0)]
    lights: [Vec4; MAX_LIGHTS],
    #[uniform(0)]
    colors: [Vec4; MAX_LIGHTS],
}

impl Material2d for DarknessMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://undercroft/shaders/darkness.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component)]
struct DarknessOverlay;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/darkness.wgsl");
        app.add_plugins(Material2dPlugin::<DarknessMaterial>::default())
            .add_systems(Startup, spawn_overlay)
            .add_systems(
                PostUpdate,
                update_lights.before(TransformSystems::Propagate),
            );
    }
}

fn spawn_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DarknessMaterial>>,
) {
    let material = materials.add(DarknessMaterial {
        params: Vec4::new(0.9, 0.6, 0.0, 0.0),
        lights: [Vec4::ZERO; MAX_LIGHTS],
        colors: [Vec4::ZERO; MAX_LIGHTS],
    });
    commands.spawn((
        DarknessOverlay,
        Mesh2d(meshes.add(Rectangle::new(CANVAS_W as f32 + 4.0, CANVAS_H as f32 + 4.0))),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, 0.0, layer::DARKNESS),
    ));
}

fn update_lights(
    time: Res<Time>,
    state: Res<State<GameState>>,
    floor: Option<Res<CurrentFloor>>,
    camera: Query<&Transform, (With<GameCamera>, Without<DarknessOverlay>)>,
    mut overlay: Query<(&mut Transform, &MeshMaterial2d<DarknessMaterial>), With<DarknessOverlay>>,
    lights: Query<(&GlobalTransform, &Light)>,
    mut materials: ResMut<Assets<DarknessMaterial>>,
) {
    let Ok(cam) = camera.single() else { return };
    let Ok((mut transform, handle)) = overlay.single_mut() else {
        return;
    };
    transform.translation = cam.translation.truncate().extend(layer::DARKNESS);
    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };

    let t = time.elapsed_secs();
    let cam_pos = cam.translation.truncate();
    let mut entries: Vec<(f32, Vec4, Vec4)> = lights
        .iter()
        .map(|(gt, light)| {
            let pos = gt.translation().truncate();
            let phase = pos.x * 0.37 + pos.y * 0.61;
            let flicker = 1.0
                + light.flicker
                    * ((t * 9.0 + phase).sin() * 0.6 + (t * 23.0 + phase * 2.0).sin() * 0.4);
            let rgb = light.color.to_linear();
            (
                pos.distance_squared(cam_pos),
                Vec4::new(pos.x, pos.y, light.radius * flicker, light.intensity),
                Vec4::new(rgb.red, rgb.green, rgb.blue, 1.0),
            )
        })
        .collect();
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    let count = entries.len().min(MAX_LIGHTS);
    for (i, (_, l, c)) in entries.iter().take(count).enumerate() {
        material.lights[i] = *l;
        material.colors[i] = *c;
    }
    let ambient = match (state.get(), &floor) {
        (
            GameState::Title | GameState::ClassSelect | GameState::GameOver | GameState::Victory,
            _,
        ) => 0.0,
        (_, Some(f)) => (0.82 + f.dungeon.floor as f32 * 0.015).min(0.94),
        _ => 0.85,
    };
    material.params = Vec4::new(ambient, 0.7, t, count as f32);
}
