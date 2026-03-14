// Checkerboard + canvas render shader
// Draws a transparency checkerboard, then composites the canvas image on top.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct CanvasParams {
    // Canvas rect in clip space (NDC)
    canvas_min: vec2<f32>,
    canvas_max: vec2<f32>,
    // Viewport size in pixels
    viewport_size: vec2<f32>,
    // Checkerboard tile size in pixels
    tile_size: f32,
    _pad: f32,
}

@group(0) @binding(0) var canvas_tex: texture_2d<f32>;
@group(0) @binding(1) var canvas_sampler: sampler;
@group(0) @binding(2) var<uniform> params: CanvasParams;

// Full-screen triangle
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    // Generate a full-screen triangle from vertex index
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // UV: map from clip [-1,1] to [0,1]
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = in.uv * params.viewport_size;

    // Map from viewport UV to canvas UV
    let canvas_uv = (in.uv - params.canvas_min) / (params.canvas_max - params.canvas_min);

    // Check if we're inside the canvas rect
    let inside = canvas_uv.x >= 0.0 && canvas_uv.x <= 1.0 &&
                 canvas_uv.y >= 0.0 && canvas_uv.y <= 1.0;

    if !inside {
        // Outside canvas: dark gray workspace background
        return vec4<f32>(0.2, 0.2, 0.2, 1.0);
    }

    // Checkerboard pattern (transparency indicator)
    let checker_coord = vec2<i32>(floor(pixel / params.tile_size));
    let is_light = (checker_coord.x + checker_coord.y) % 2 == 0;
    let checker_color = select(vec3<f32>(0.6), vec3<f32>(0.8), is_light);

    // Sample the canvas texture
    let tex_color = textureSample(canvas_tex, canvas_sampler, canvas_uv);

    // Composite: canvas over checkerboard
    let out_rgb = tex_color.rgb * tex_color.a + checker_color * (1.0 - tex_color.a);

    return vec4<f32>(out_rgb, 1.0);
}
