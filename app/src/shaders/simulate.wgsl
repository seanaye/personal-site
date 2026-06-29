struct SimParams {
    width: u32,
    height: u32,
    damping: f32,
    _pad: f32,
}

struct Drops {
    count: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    coords: array<vec4<u32>, 16>,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<uniform> drops: Drops;

@vertex
fn vs(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4f {
    let x = f32(i32(idx & 1u) * 4 - 1);
    let y = f32(i32(idx & 2u) * 2 - 1);
    return vec4f(x, y, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) frag_coord: vec4f) -> @location(0) vec2f {
    let x = u32(frag_coord.x);
    let y = u32(frag_coord.y);
    let w = params.width;
    let h = params.height;

    // Check if this pixel is in a 2x2 drop block. Add the impulse to the
    // existing height instead of replacing it, preserving overlapping waves.
    for (var i = 0u; i < drops.count; i++) {
        let drop = drops.coords[i];
        if ((x == drop.x || x == drop.x + 1u) && (y == drop.y || y == drop.y + 1u)) {
            let old = textureLoad(input_tex, vec2i(i32(x), i32(y)), 0);
            return vec2f(old.r + 256.0, old.r);
        }
    }

    // Sample 4 neighbors' R channel (current state)
    var sum: f32 = 0.0;
    if (x > 0u)     { sum += textureLoad(input_tex, vec2i(i32(x) - 1, i32(y)), 0).r; }
    if (x < w - 1u) { sum += textureLoad(input_tex, vec2i(i32(x) + 1, i32(y)), 0).r; }
    if (y > 0u)     { sum += textureLoad(input_tex, vec2i(i32(x), i32(y) - 1), 0).r; }
    if (y < h - 1u) { sum += textureLoad(input_tex, vec2i(i32(x), i32(y) + 1), 0).r; }

    // Read current pixel (R = current state, G = state from 2 frames ago)
    let old = textureLoad(input_tex, vec2i(i32(x), i32(y)), 0);

    sum = sum / 2.0;
    sum -= old.g;
    sum *= params.damping;

    // Output: R = new state, G = old R (shift history forward)
    return vec2f(sum, old.r);
}
