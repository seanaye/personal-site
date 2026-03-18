@group(0) @binding(0) var sim_tex: texture_2d<f32>;
@group(0) @binding(1) var palette_tex: texture_2d<f32>;

@vertex
fn vs(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4f {
    let x = f32(i32(idx & 1u) * 4 - 1);
    let y = f32(i32(idx & 2u) * 2 - 1);
    return vec4f(x, y, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) frag_coord: vec4f) -> @location(0) vec4f {
    let coord = vec2i(frag_coord.xy);
    let value = textureLoad(sim_tex, coord, 0).r;

    let palette_idx = clamp(i32(value + 128.0), 0, 255);
    let color = textureLoad(palette_tex, vec2i(palette_idx, 0), 0);
    return vec4f(color.rgb, 1.0);
}
