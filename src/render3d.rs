//! 3D rendering pipeline with perspective camera and depth buffer.
//!
//! Extends the 2D renderer with 3D mesh support including:
//! - Perspective projection camera
//! - Depth/stencil buffer
//! - 3D mesh vertex layout
//! - World-space transforms (position, rotation, scale)

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device,
    Extent3d, FragmentState, FrontFace, IndexFormat, LoadOp, MultisampleState,
    Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, Queue,
    ShaderStages, BufferBindingType,
    CompareFunction, DepthStencilState, StencilState, StencilFaceState,
    ColorTargetState, ColorWrites, BlendState,
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub u: f32,
    pub v: f32,
}

impl Vertex3D {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vertex3D { x, y, z, nx: 0.0, ny: 1.0, nz: 0.0, u: 0.0, v: 0.0 }
    }

    pub fn with_normal(mut self, nx: f32, ny: f32, nz: f32) -> Self {
        self.nx = nx;
        self.ny = ny;
        self.nz = nz;
        self
    }

    pub fn with_uv(mut self, u: f32, v: f32) -> Self {
        self.u = u;
        self.v = v;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Mesh3D {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
}

impl Mesh3D {
    pub fn new(vertices: Vec<Vertex3D>, indices: Vec<u32>) -> Self {
        Mesh3D { vertices, indices }
    }

    pub fn cube() -> Self {
        let h = 0.5f32;
        let verts = vec![
            // Front face (z+)
            Vertex3D::new(-h, -h,  h).with_normal( 0.0,  0.0,  1.0).with_uv(0.0, 1.0),
            Vertex3D::new( h, -h,  h).with_normal( 0.0,  0.0,  1.0).with_uv(1.0, 1.0),
            Vertex3D::new( h,  h,  h).with_normal( 0.0,  0.0,  1.0).with_uv(1.0, 0.0),
            Vertex3D::new(-h,  h,  h).with_normal( 0.0,  0.0,  1.0).with_uv(0.0, 0.0),
            // Back face (z-)
            Vertex3D::new(-h, -h, -h).with_normal( 0.0,  0.0, -1.0).with_uv(1.0, 1.0),
            Vertex3D::new( h, -h, -h).with_normal( 0.0,  0.0, -1.0).with_uv(0.0, 1.0),
            Vertex3D::new( h,  h, -h).with_normal( 0.0,  0.0, -1.0).with_uv(0.0, 0.0),
            Vertex3D::new(-h,  h, -h).with_normal( 0.0,  0.0, -1.0).with_uv(1.0, 0.0),
            // Top face (y+)
            Vertex3D::new(-h,  h, -h).with_normal( 0.0,  1.0,  0.0).with_uv(0.0, 1.0),
            Vertex3D::new( h,  h, -h).with_normal( 0.0,  1.0,  0.0).with_uv(1.0, 1.0),
            Vertex3D::new( h,  h,  h).with_normal( 0.0,  1.0,  0.0).with_uv(1.0, 0.0),
            Vertex3D::new(-h,  h,  h).with_normal( 0.0,  1.0,  0.0).with_uv(0.0, 0.0),
            // Bottom face (y-)
            Vertex3D::new(-h, -h, -h).with_normal( 0.0, -1.0,  0.0).with_uv(0.0, 0.0),
            Vertex3D::new( h, -h, -h).with_normal( 0.0, -1.0,  0.0).with_uv(1.0, 0.0),
            Vertex3D::new( h, -h,  h).with_normal( 0.0, -1.0,  0.0).with_uv(1.0, 1.0),
            Vertex3D::new(-h, -h,  h).with_normal( 0.0, -1.0,  0.0).with_uv(0.0, 1.0),
            // Right face (x+)
            Vertex3D::new( h, -h, -h).with_normal( 1.0,  0.0,  0.0).with_uv(0.0, 1.0),
            Vertex3D::new( h,  h, -h).with_normal( 1.0,  0.0,  0.0).with_uv(0.0, 0.0),
            Vertex3D::new( h,  h,  h).with_normal( 1.0,  0.0,  0.0).with_uv(1.0, 0.0),
            Vertex3D::new( h, -h,  h).with_normal( 1.0,  0.0,  0.0).with_uv(1.0, 1.0),
            // Left face (x-)
            Vertex3D::new(-h, -h, -h).with_normal(-1.0,  0.0,  0.0).with_uv(1.0, 1.0),
            Vertex3D::new(-h,  h, -h).with_normal(-1.0,  0.0,  0.0).with_uv(1.0, 0.0),
            Vertex3D::new(-h,  h,  h).with_normal(-1.0,  0.0,  0.0).with_uv(0.0, 0.0),
            Vertex3D::new(-h, -h,  h).with_normal(-1.0,  0.0,  0.0).with_uv(0.0, 1.0),
        ];

        let indices = vec![
            0,  1,  2,  2,  3,  0,
            4,  5,  6,  6,  7,  4,
            8,  9, 10, 10, 11,  8,
            12, 13, 14, 14, 15, 12,
            16, 17, 18, 18, 19, 16,
            20, 21, 22, 22, 23, 20,
        ];

        Mesh3D::new(verts, indices)
    }

    pub fn plane() -> Self {
        let h = 0.5f32;
        let verts = vec![
            Vertex3D::new(-h, 0.0, -h).with_normal(0.0, 1.0, 0.0).with_uv(0.0, 1.0),
            Vertex3D::new( h, 0.0, -h).with_normal(0.0, 1.0, 0.0).with_uv(1.0, 1.0),
            Vertex3D::new( h, 0.0,  h).with_normal(0.0, 1.0, 0.0).with_uv(1.0, 0.0),
            Vertex3D::new(-h, 0.0,  h).with_normal(0.0, 1.0, 0.0).with_uv(0.0, 0.0),
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];
        Mesh3D::new(verts, indices)
    }

    pub fn upload_vertex_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh-vertex-buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: BufferUsages::VERTEX,
        })
    }

    pub fn upload_index_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh-index-buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: BufferUsages::INDEX,
        })
    }

    pub fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transform3D {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Transform3D {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Transform3D {
            position: [x, y, z],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn with_rotation(mut self, rx: f32, ry: f32, rz: f32) -> Self {
        self.rotation = [rx, ry, rz];
        self
    }

    pub fn with_scale(mut self, sx: f32, sy: f32, sz: f32) -> Self {
        self.scale = [sx, sy, sz];
        self
    }

    pub fn to_matrix(&self) -> [[f32; 4]; 4] {
        let [sx, sy, sz] = self.scale;
        let [rx, ry, rz] = self.rotation;
        let [px, py, pz] = self.position;

        let cx = rx.cos(); let sxr = rx.sin();
        let cy = ry.cos(); let syr = ry.sin();
        let cz = rz.cos(); let szr = rz.sin();

        // Scale * RotationZ * RotationY * RotationX * Translation
        let s00 = sx * (cy * cz);
        let s01 = sx * (cy * szr);
        let s02 = sx * (-syr);
        let s10 = sy * (sxr * syr * cz - cx * szr);
        let s11 = sy * (sxr * syr * szr + cx * cz);
        let s12 = sy * (sxr * cy);
        let s20 = sz * (cx * syr * cz + sxr * szr);
        let s21 = sz * (cx * syr * szr - sxr * cz);
        let s22 = sz * (cx * cy);

        [
            [s00, s10, s20, 0.0],
            [s01, s11, s21, 0.0],
            [s02, s12, s22, 0.0],
            [px,  py,  pz,  1.0],
        ]
    }
}

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveCamera {
    pub fn new(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        PerspectiveCamera {
            position: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov,
            aspect,
            near,
            far,
        }
    }

    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let f = [
            self.target[0] - self.position[0],
            self.target[1] - self.position[1],
            self.target[2] - self.position[2],
        ];
        let fl = (f[0]*f[0] + f[1]*f[1] + f[2]*f[2]).sqrt();
        let f = [f[0]/fl, f[1]/fl, f[2]/fl];

        let s = [
            f[1]*self.up[2] - f[2]*self.up[1],
            f[2]*self.up[0] - f[0]*self.up[2],
            f[0]*self.up[1] - f[1]*self.up[0],
        ];
        let sl = (s[0]*s[0] + s[1]*s[1] + s[2]*s[2]).sqrt();
        let s = [s[0]/sl, s[1]/sl, s[2]/sl];

        let u = [
            s[1]*f[2] - s[2]*f[1],
            s[2]*f[0] - s[0]*f[2],
            s[0]*f[1] - s[1]*f[0],
        ];

        let p = self.position;
        [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [-(s[0]*p[0]+s[1]*p[1]+s[2]*p[2]),
             -(u[0]*p[0]+u[1]*p[1]+u[2]*p[2]),
             (f[0]*p[0]+f[1]*p[1]+f[2]*p[2]),
             1.0],
        ]
    }

    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        let f = 1.0 / (self.fov / 2.0).tan();
        let range_inv = 1.0 / (self.near - self.far);

        [
            [f / self.aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, self.far * range_inv, -1.0],
            [0.0, 0.0, self.near * self.far * range_inv, 0.0],
        ]
    }

    pub fn vp_matrix(&self) -> [[f32; 4]; 4] {
        multiply_matrices(&self.projection_matrix(), &self.view_matrix())
    }

    pub fn look_at(&mut self, eye: [f32; 3], target: [f32; 3]) {
        self.position = eye;
        self.target = target;
    }
}

fn multiply_matrices(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = a[0][j]*b[i][0] + a[1][j]*b[i][1]
                         + a[2][j]*b[i][2] + a[3][j]*b[i][3];
        }
    }
    result
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

fn create_depth_texture(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("depth-texture"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

const SHADER_3D_WGSL: &str = r#"
struct CameraUniform {
    vp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct MeshInstance {
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct VSInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VSOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

fn instance_matrix(inst: MeshInstance) -> mat4x4<f32> {
    return mat4x4<f32>(
        inst.model_0,
        inst.model_1,
        inst.model_2,
        inst.model_3,
    );
}

@vertex
fn vs(input: VSInput, inst: MeshInstance) -> VSOutput {
    let model = instance_matrix(inst);
    let world_pos = model * vec4<f32>(input.pos, 1.0);
    let clip_pos = camera.vp * world_pos;
    let world_normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    return VSOutput(clip_pos, world_normal, inst.color);
}

@fragment
fn fs(input: VSOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(input.normal, light_dir), 0.0) * 0.7;
    let brightness = ambient + diffuse;
    return vec4<f32>(input.color.rgb * brightness, input.color.a);
}
"#;

pub struct Renderer3D {
    pub device: Device,
    pub queue: Queue,
    pipeline: RenderPipeline,
    depth_texture: Texture,
    depth_view: TextureView,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    pub width: u32,
    pub height: u32,
}

impl Renderer3D {
    pub fn new(device: Device, queue: Queue, width: u32, height: u32) -> Self {
        let (depth_texture, depth_view) = create_depth_texture(&device, width, height);

        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("3d-camera-bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("3d-camera-buffer"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("3d-camera-bg"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render3d.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER_3D_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("3d-pipeline-layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("3d-render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex3D>() as u64,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[
                            VertexAttribute { format: VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                            VertexAttribute { format: VertexFormat::Float32x3, offset: 12, shader_location: 1 },
                        ],
                    },
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<MeshInstance>() as u64,
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute { format: VertexFormat::Float32x4, offset: 0, shader_location: 2 },
                            VertexAttribute { format: VertexFormat::Float32x4, offset: 16, shader_location: 3 },
                            VertexAttribute { format: VertexFormat::Float32x4, offset: 32, shader_location: 4 },
                            VertexAttribute { format: VertexFormat::Float32x4, offset: 48, shader_location: 5 },
                            VertexAttribute { format: VertexFormat::Float32x4, offset: 64, shader_location: 6 },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Renderer3D {
            device,
            queue,
            pipeline,
            depth_texture,
            depth_view,
            camera_buffer,
            camera_bind_group,
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let (dt, dv) = create_depth_texture(&self.device, width, height);
        self.depth_texture = dt;
        self.depth_view = dv;
    }

    pub fn update_camera(&mut self, camera: &PerspectiveCamera) {
        let vp = camera.vp_matrix();
        let flat: [f32; 16] = [
            vp[0][0], vp[1][0], vp[2][0], vp[3][0],
            vp[0][1], vp[1][1], vp[2][1], vp[3][1],
            vp[0][2], vp[1][2], vp[2][2], vp[3][2],
            vp[0][3], vp[1][3], vp[2][3], vp[3][3],
        ];
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&flat));
    }

    pub fn upload_instances(&self, transforms: &[Transform3D], colors: &[[f32; 4]]) -> Buffer {
        let instances: Vec<MeshInstance> = transforms.iter().zip(colors.iter()).map(|(t, c)| {
            let m = t.to_matrix();
            MeshInstance {
                model: m,
                color: *c,
            }
        }).collect();
        self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("mesh-instance-buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: BufferUsages::VERTEX,
        })
    }

    pub fn render_mesh(
        &self,
        view: &TextureView,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
        index_count: u32,
        instance_buffer: &Buffer,
        instance_count: u32,
        clear_color: wgpu::Color,
    ) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("3d-render-encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("3d-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, instance_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..index_count, 0, 0..instance_count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
