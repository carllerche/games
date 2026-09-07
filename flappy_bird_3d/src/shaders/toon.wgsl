// Cel shader: two hard lighting bands plus a subtle rim highlight.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct ToonMaterial {
    color: vec4<f32>,
    // xyz: direction toward the light, w: shadow brightness.
    light: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: ToonMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let l = normalize(material.light.xyz);
    let n_dot_l = dot(n, l);

    // Two hard bands with a tiny blend so the edge doesn't shimmer.
    let lit = smoothstep(0.05, 0.11, n_dot_l);
    let shade = mix(material.light.w, 1.0, lit);
    var rgb = material.color.rgb * shade;

    // A faint rim on the lit side lifts silhouettes off the background.
    let v = normalize(view.world_position - in.world_position.xyz);
    let rim = 1.0 - max(dot(n, v), 0.0);
    rgb += material.color.rgb * 0.16 * smoothstep(0.62, 0.72, rim) * lit;

    return vec4<f32>(rgb, material.color.a);
}
