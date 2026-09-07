// Full-screen darkness overlay with point lights, quantized and dithered so
// the lighting matches the chunky pixel art underneath.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Darkness {
    // x: ambient darkness, y: vignette strength, z: time, w: light count.
    params: vec4<f32>,
    // xy: world position, z: radius, w: intensity.
    lights: array<vec4<f32>, 48>,
    colors: array<vec4<f32>, 48>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: Darkness;

fn bayer(p: vec2<f32>) -> f32 {
    let x = i32(p.x) & 3;
    let y = i32(p.y) & 3;
    let index = y * 4 + x;
    var m = array<f32, 16>(
        0.0, 8.0, 2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0, 1.0, 9.0,
        15.0, 7.0, 13.0, 5.0,
    );
    return (m[index] + 0.5) / 16.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xy;
    var light = 0.0;
    var tint = vec3<f32>(0.0);
    let count = i32(material.params.w);
    for (var i = 0; i < count; i++) {
        let l = material.lights[i];
        let d = distance(p, l.xy);
        let falloff = 1.0 - smoothstep(l.z * 0.1, l.z, d);
        let c = falloff * falloff * l.w;
        light += c;
        tint += material.colors[i].rgb * c;
    }
    let raw = light;
    light = clamp(light, 0.0, 1.0);

    // Ordered dither before quantizing into five brightness bands.
    let levels = 5.0;
    let dithered = light + (bayer(in.position.xy) - 0.5) * 0.7 / levels;
    let q = clamp(floor(dithered * levels + 0.5) / levels, 0.0, 1.0);

    let ambient = material.params.x;
    var alpha = ambient * (1.0 - q);

    let uv = in.uv - vec2<f32>(0.5);
    let v = dot(uv, uv) * 3.0;
    alpha = clamp(alpha + material.params.y * v * v, 0.0, 1.0);

    // Lit areas take on a hint of the light colour, dark areas go cold.
    let warm = select(vec3<f32>(0.0), tint / max(raw, 0.001), raw > 0.0);
    let base = vec3<f32>(0.02, 0.012, 0.05);
    let color = mix(base, warm * 0.35, q * 0.5);
    return vec4<f32>(color, alpha);
}
