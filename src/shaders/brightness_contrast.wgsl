// Brightness/Contrast adjustment compute shader

struct Params {
    width: u32,
    height: u32,
    brightness: f32, // -1.0 to 1.0
    contrast: f32,   // -1.0 to 1.0
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var out_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= params.width || gid.y >= params.height {
        return;
    }

    let coords = vec2<i32>(i32(gid.x), i32(gid.y));
    let pixel = textureLoad(src_tex, coords, 0);

    // Brightness: simple add
    var rgb = pixel.rgb + vec3<f32>(params.brightness);

    // Contrast: scale around 0.5
    let factor = (1.0 + params.contrast);
    rgb = (rgb - vec3<f32>(0.5)) * factor + vec3<f32>(0.5);

    // Clamp
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));

    textureStore(out_tex, coords, vec4<f32>(rgb, pixel.a));
}
