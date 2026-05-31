//! Post-processing effects via render-to-texture.

#[cfg(feature = "render")]
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferDescriptor, BufferUsages, ColorTargetState,
    ColorWrites, CommandEncoderDescriptor, Device, Extent3d, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

#[cfg(feature = "render")]
use wgpu::util::{BufferInitDescriptor, DeviceExt};

#[derive(Debug, Clone, Copy)]
pub struct ColorGradeParams {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub tint_r: f32,
    pub tint_g: f32,
    pub tint_b: f32,
    pub vignette_strength: f32,
    pub vignette_radius: f32,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
}

impl Default for ColorGradeParams {
    fn default() -> Self {
        ColorGradeParams {
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            vignette_strength: 0.0,
            vignette_radius: 0.8,
            bloom_threshold: 0.8,
            bloom_intensity: 0.0,
        }
    }
}

impl ColorGradeParams {
    pub fn standard() -> Self {
        ColorGradeParams::default()
    }

    pub fn crt() -> Self {
        ColorGradeParams {
            brightness: 1.1,
            contrast: 1.3,
            saturation: 0.85,
            gamma: 0.95,
            tint_r: 1.0,
            tint_g: 0.95,
            tint_b: 0.9,
            vignette_strength: 0.6,
            vignette_radius: 0.7,
            bloom_threshold: 0.7,
            bloom_intensity: 0.15,
        }
    }

    pub fn noir() -> Self {
        ColorGradeParams {
            brightness: 1.0,
            contrast: 1.4,
            saturation: 0.0,
            gamma: 0.9,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            vignette_strength: 0.8,
            vignette_radius: 0.6,
            bloom_threshold: 0.9,
            bloom_intensity: 0.05,
        }
    }

    pub fn sunset() -> Self {
        ColorGradeParams {
            brightness: 1.05,
            contrast: 1.1,
            saturation: 1.3,
            gamma: 1.0,
            tint_r: 1.1,
            tint_g: 0.9,
            tint_b: 0.7,
            vignette_strength: 0.3,
            vignette_radius: 0.85,
            bloom_threshold: 0.6,
            bloom_intensity: 0.2,
        }
    }
}

#[cfg(feature = "render")]
const POSTPROCESS_WGSL: &str = r#"
struct Params {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    gamma: f32,
    tint: vec3<f32>,
    vignette_strength: f32,
    vignette_radius: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
};

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var src_texture: texture_2d<f32>;

@group(1) @binding(1)
var src_sampler: sampler;

struct VSOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VSOutput {
    return VSOutput(vec4<f32>(position, 0.0, 1.0), uv);
}

@fragment
fn fs(input: VSOutput) -> @location(0) vec4<f32> {
    var color = textureSample(src_texture, src_sampler, input.uv).rgb;

    color = color * params.brightness;
    color = (color - 0.5) * params.contrast + 0.5;

    let lum = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    color = mix(vec3<f32>(lum), color, params.saturation);

    color = color * params.tint;

    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / params.gamma));

    let center = input.uv - vec2<f32>(0.5);
    let dist = length(center);
    let vignette = smoothstep(params.vignette_radius, params.vignette_radius - 0.3, dist);
    color = color * mix(1.0, vignette, params.vignette_strength);

    let frag_lum = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    if frag_lum > params.bloom_threshold {
        let bloom = (frag_lum - params.bloom_threshold) / (1.0 - params.bloom_threshold);
        color = color + color * bloom * params.bloom_intensity;
    }

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

#[cfg(feature = "render")]
const FULLSCREEN_VERTICES: &[f32] = &[
    -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
];

#[cfg(feature = "render")]
const FULLSCREEN_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[cfg(feature = "render")]
pub struct PostProcessor {
    pipeline: RenderPipeline,
    params_buffer: Buffer,
    _params_bind_group: BindGroup,
    params_bind_group_layout: BindGroupLayout,
    texture_bind_group_layout: BindGroupLayout,
    render_texture: Texture,
    render_view: TextureView,
    sampler: Sampler,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    pub width: u32,
    pub height: u32,
    pub params: ColorGradeParams,
}

#[cfg(feature = "render")]
impl PostProcessor {
    pub fn new(device: &Device, width: u32, height: u32, format: TextureFormat) -> Self {
        let (render_texture, render_view) =
            Self::create_render_texture(device, width, height, format);

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("postprocess-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let params_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("pp-params-bgl"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("pp-params-buffer"),
            size: 48,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("pp-params-bg"),
            layout: &params_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("pp-texture-bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("postprocess.wgsl"),
            source: wgpu::ShaderSource::Wgsl(POSTPROCESS_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pp-pipeline-layout"),
            bind_group_layouts: &[&params_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("postprocess-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[VertexBufferLayout {
                    array_stride: 16,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pp-vertex-buffer"),
            contents: bytemuck::cast_slice(FULLSCREEN_VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pp-index-buffer"),
            contents: bytemuck::cast_slice(FULLSCREEN_INDICES),
            usage: BufferUsages::INDEX,
        });

        PostProcessor {
            pipeline,
            params_buffer,
            _params_bind_group: params_bind_group,
            params_bind_group_layout,
            texture_bind_group_layout,
            render_texture,
            render_view,
            sampler,
            vertex_buffer,
            index_buffer,
            width,
            height,
            params: ColorGradeParams::standard(),
        }
    }

    fn create_render_texture(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("pp-render-texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32, format: TextureFormat) {
        self.width = width;
        self.height = height;
        let (t, v) = Self::create_render_texture(device, width, height, format);
        self.render_texture = t;
        self.render_view = v;
    }

    pub fn render_target(&self) -> &TextureView {
        &self.render_view
    }

    pub fn apply(&mut self, device: &Device, queue: &Queue, output_view: &TextureView) {
        let params_data: [f32; 12] = [
            self.params.brightness,
            self.params.contrast,
            self.params.saturation,
            self.params.gamma,
            self.params.tint_r,
            self.params.tint_g,
            self.params.tint_b,
            self.params.vignette_strength,
            self.params.vignette_radius,
            self.params.bloom_threshold,
            self.params.bloom_intensity,
            0.0,
        ];
        self.params_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pp-params-upload"),
            contents: bytemuck::cast_slice(&params_data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let params_bg = device.create_bind_group(&BindGroupDescriptor {
            label: Some("pp-params-bg"),
            layout: &self.params_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.params_buffer.as_entire_binding(),
            }],
        });

        let texture_bg = device.create_bind_group(&BindGroupDescriptor {
            label: Some("pp-texture-bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.render_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("pp-encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pp-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &params_bg, &[]);
            pass.set_bind_group(1, &texture_bg, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}
