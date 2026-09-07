//! Cel-shaded material: hard two-band lighting with a soft rim, in the style
//! of The Legend of Zelda: The Wind Waker. It ignores Bevy's lights and uses
//! a single fixed light direction so every object shades consistently.

use bevy::{
    asset::embedded_asset, prelude::*, reflect::TypePath, render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// Direction *toward* the sun. Up, to the right, and toward the camera so the
/// faces the player sees are lit.
pub const LIGHT_DIR: Vec3 = Vec3::new(0.45, 0.80, 0.55);
/// Brightness of the shadowed band relative to the lit band.
pub const SHADOW_BRIGHTNESS: f32 = 0.62;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ToonMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    /// xyz: direction toward the light, w: shadow brightness.
    #[uniform(0)]
    pub light: Vec4,
}

impl ToonMaterial {
    pub fn new(color: Color) -> Self {
        Self {
            color: color.into(),
            light: LIGHT_DIR.normalize().extend(SHADOW_BRIGHTNESS),
        }
    }
}

impl Material for ToonMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://flappy_bird_3d/shaders/toon.wgsl".into()
    }
}

pub struct ToonPlugin;

impl Plugin for ToonPlugin {
    fn build(&self, app: &mut App) {
        // The shader ships inside the binary, so the game runs from anywhere.
        embedded_asset!(app, "shaders/toon.wgsl");
        app.add_plugins(MaterialPlugin::<ToonMaterial>::default());
    }
}
