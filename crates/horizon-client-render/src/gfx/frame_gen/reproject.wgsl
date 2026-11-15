// Frame generation's fast path (`FrameGeneration::Fast`): the frame halfway between the last
// real frame and this one, made in one pass. The world stands still while the camera moves,
// so the depth and the camera's motion say where each pixel was a frame ago; mobs and dropped
// items carry their own motion (motion.wgsl). No optical flow and no pyramids: FSR 3.1 runs
// some sixty passes at the screen's size, which on a phone cost more than the frame itself.
//
// A pixel at the halfway point `m` shows the surface that is at `c` this frame and was at
// `c + v(c)` the frame before, with `m = c + v(c) / 2` (`v`: the motion vector toward the last
// frame, FSR's convention). `c` is found from `m` by two steps of `c = m - v(c) / 2`, and the
// made pixel blends this frame at `c` with the last at `m + v(c) / 2`. Where the two steps
// disagree, an edge or a surface that just came into view, the last frame's pixel is likely
// another surface, so this frame's alone is shown.

struct Params {
    // This frame's unjittered clip space to the previous frame's.
    reproject: mat4x4<f32>,
    // The depth's size, and TAA's jitter this frame in its pixels.
    render_size: vec2<f32>,
    jitter: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var linear_clamp: sampler;
// The world's depth at the render size, reversed (1 near, 0 far), bound as unfilterable
// float as motion.wgsl does.
@group(0) @binding(2) var depth: texture_2d<f32>;
// This frame's world and the last one's, at the screen's size, without the hand and HUD.
@group(0) @binding(3) var current_color: texture_2d<f32>;
@group(0) @binding(4) var previous_color: texture_2d<f32>;
// What moved on its own (motion.wgsl), at the render size: xy its motion where z is 1.
@group(0) @binding(5) var object_motion: texture_2d<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle over the whole target.
@vertex fn vs_full(@builtin(vertex_index) i: u32) -> VsOut {
    let xy = vec2(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4(xy * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(xy.x, 1.0 - xy.y);
    return out;
}

fn render_texel(uv: vec2<f32>) -> vec2<i32> {
    let size = params.render_size;
    let p = vec2<i32>(floor(uv * size + params.jitter));
    return clamp(p, vec2(0), vec2<i32>(size) - 1);
}

// The nearest depth around a texel (the largest, reversed): an edge takes the motion of what
// is in front, so a mob's or a block's outline moves with it and not with the sky behind.
fn nearest_depth(p: vec2<i32>) -> f32 {
    let last = vec2<i32>(params.render_size) - 1;
    var d = textureLoad(depth, p, 0).r;
    d = max(d, textureLoad(depth, clamp(p + vec2(1, 0), vec2(0), last), 0).r);
    d = max(d, textureLoad(depth, clamp(p - vec2(1, 0), vec2(0), last), 0).r);
    d = max(d, textureLoad(depth, clamp(p + vec2(0, 1), vec2(0), last), 0).r);
    d = max(d, textureLoad(depth, clamp(p - vec2(0, 1), vec2(0), last), 0).r);
    return d;
}

// The motion vector of what this frame shows at `uv`: toward where it was a frame ago, in UV.
fn motion(uv: vec2<f32>) -> vec2<f32> {
    let p = render_texel(uv);
    let object = textureLoad(object_motion, p, 0);
    if object.z > 0.5 {
        return object.xy;
    }
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let previous = params.reproject * vec4(ndc, nearest_depth(p), 1.0);
    if previous.w <= 0.0 {
        return vec2(0.0);
    }
    let previous_uv = vec2(0.5, -0.5) * previous.xy / previous.w + 0.5;
    return previous_uv - uv;
}

fn inside(uv: vec2<f32>) -> bool {
    return all(uv >= vec2(0.0)) && all(uv <= vec2(1.0));
}

@fragment fn fs_made(in: VsOut) -> @location(0) vec4<f32> {
    let m = in.uv;
    let first = motion(m);
    let v = motion(m - 0.5 * first);
    let c = m - 0.5 * v;
    let p = m + 0.5 * v;
    let now = textureSampleLevel(current_color, linear_clamp, c, 0.0).rgb;
    let then = textureSampleLevel(previous_color, linear_clamp, p, 0.0).rgb;
    // How many screen pixels the two steps disagree by: past a few, trust this frame alone.
    let screen = vec2<f32>(textureDimensions(current_color));
    let disagreement = length((v - first) * screen);
    var weight = 0.5 * (1.0 - smoothstep(1.0, 4.0, disagreement));
    if !inside(p) {
        weight = 0.0;
    } else if !inside(c) {
        weight = 1.0;
    }
    return vec4(mix(now, then, weight), 1.0);
}
