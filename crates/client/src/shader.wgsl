struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    fog_color: vec4<f32>,
    // x = fog start, y = fog end, z = 1.0 if the surface is sRGB
    fog: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = globals.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let d = distance(in.world_pos.xz, globals.camera_pos.xz);
    let f = clamp((d - globals.fog.x) / (globals.fog.y - globals.fog.x), 0.0, 1.0);
    let color = mix(in.color, globals.fog_color.rgb, f);
    // Palette colours are authored in sRGB; an sRGB surface expects linear output.
    if globals.fog.z > 0.5 {
        return vec4<f32>(pow(color, vec3<f32>(2.2)), 1.0);
    }
    return vec4<f32>(color, 1.0);
}
