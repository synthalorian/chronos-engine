struct CameraUniform {
    proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var my_texture: texture_2d<f32>;

@group(1) @binding(1)
var my_sampler: sampler;

struct QuadVertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct SpriteInstance {
    @location(2) x: f32,
    @location(3) y: f32,
    @location(4) width: f32,
    @location(5) height: f32,
    @location(6) uv_rect: vec4<f32>,
    @location(7) layer: f32,
    @location(8) color: vec4<f32>,
    @location(9) parallax: f32,
};

struct VSOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(qv: QuadVertex, inst: SpriteInstance) -> VSOutput {
    let half_w = inst.width * 0.5;
    let half_h = inst.height * 0.5;

    // Parallax-adjusted world position
    let world_pos = vec2<f32>(
        inst.x + qv.pos.x * half_w,
        inst.y + qv.pos.y * half_h,
    );

    // Scale camera translation by parallax factor (1.0 = normal, 0.0 = pinned)
    var adjusted_proj = camera.proj;
    // Columns 0,1,2 keep their scale/rotation; column 3 has translation.
    // Translation lives at [3][0] and [3][1] (row-major mat4x4).
    adjusted_proj[3][0] = adjusted_proj[3][0] * inst.parallax;
    adjusted_proj[3][1] = adjusted_proj[3][1] * inst.parallax;

    let clip_pos = adjusted_proj * vec4<f32>(world_pos.x, world_pos.y, 0.0, 1.0);

    let final_uv = qv.uv * inst.uv_rect.zw + inst.uv_rect.xy;

    return VSOutput(
        clip_pos,
        final_uv,
        inst.color,
    );
}

struct FSInput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs(inp: FSInput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(my_texture, my_sampler, inp.uv);
    return tex_color * inp.color;
}
