// Layer compositing compute shader
// Blends a source layer onto a destination using the specified blend mode and opacity.

struct Params {
    width: u32,
    height: u32,
    opacity: f32,
    blend_mode: u32, // 0=Normal, 1=Multiply, 2=Screen, 3=Overlay, 4=Darken, 5=Lighten
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_2d<f32>;
@group(0) @binding(2) var out_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> params: Params;

fn blend_normal(src: vec3<f32>, dst: vec3<f32>) -> vec3<f32> {
    return src;
}

fn blend_multiply(src: vec3<f32>, dst: vec3<f32>) -> vec3<f32> {
    return src * dst;
}

fn blend_screen(src: vec3<f32>, dst: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(1.0) - (vec3<f32>(1.0) - src) * (vec3<f32>(1.0) - dst);
}

fn blend_overlay(src: vec3<f32>, dst: vec3<f32>) -> vec3<f32> {
    let lo = 2.0 * src * dst;
    let hi = vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - src) * (vec3<f32>(1.0) - dst);
    return select(hi, lo, dst < vec3<f32>(0.5));
}

fn blend_darken(src: vec3<f32>, dst: vec3<f32>) -> vec3<f32> {
    return min(src, dst);
}

fn blend_lighten(src: vec3<f32>, dst: vec3<f32>) -> vec3<f32> {
    return max(src, dst);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= params.width || gid.y >= params.height {
        return;
    }

    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    let src = textureLoad(src_tex, coords, 0);
    let dst = textureLoad(dst_tex, coords, 0);

    var blended: vec3<f32>;
    switch params.blend_mode {
        case 1u: { blended = blend_multiply(src.rgb, dst.rgb); }
        case 2u: { blended = blend_screen(src.rgb, dst.rgb); }
        case 3u: { blended = blend_overlay(src.rgb, dst.rgb); }
        case 4u: { blended = blend_darken(src.rgb, dst.rgb); }
        case 5u: { blended = blend_lighten(src.rgb, dst.rgb); }
        default: { blended = blend_normal(src.rgb, dst.rgb); }
    }

    // Alpha compositing: src over dst
    let src_a = src.a * params.opacity;
    let out_rgb = blended * src_a + dst.rgb * (1.0 - src_a);
    let out_a = src_a + dst.a * (1.0 - src_a);

    textureStore(out_tex, coords, vec4<f32>(out_rgb, out_a));
}
