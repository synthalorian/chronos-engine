//! Chronos Engine — 2D Rendering Backend (wgpu 23)
//!
//! Minimal sprite-batch renderer with orthographic camera.
//!
//! This module provides:
//! - `Camera` — orthographic camera with position, zoom, rotation
//! - `RenderSprite` — GPU-friendly sprite definition
//! - `SpriteBatch` — batched textured quad rendering via instanced rendering
//! - `Renderer` — wgpu device/surface pipeline with swapchain management
//!
//! Usage:
//! ```ignore
//! let mut renderer = Renderer::new(&window, width, height).await.unwrap();
//! let camera = Camera::new(200.0, 200.0);
//! let mut batch = SpriteBatch::new(&renderer.device, 1024);
//! renderer.render(&camera, &batch, &texture_view, &sampler);
//! ```

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompositeAlphaMode, Device, FragmentState, FrontFace,
    IndexFormat, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PresentMode,
    PrimitiveState, PrimitiveTopology, RenderPass, RenderPassColorAttachment, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, StoreOp, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, Queue, RenderPassDescriptor,
    DeviceDescriptor, ShaderStages, BufferBindingType, TextureViewDimension, TextureSampleType,
    BlendState,
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::BindingResource;

#[cfg(not(feature = "render"))]
compile_error!("render.rs should not compile without the 'render' feature");

// ──────────────────────────────────────────────
// Camera — Orthographic 2D Camera
// ──────────────────────────────────────────────

/// Orthographic camera for 2D rendering.
///
/// The camera operates in world coordinates. By default, the world
/// origin (0, 0) is centered in the viewport. Position shifts the
/// viewport, zoom scales everything, and rotation rotates around
/// the camera's position.
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: [f32; 2],
    pub zoom: f32,
    pub rotation: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    shake_intensity: f32,
    shake_decay: f32,
    shake_offset: [f32; 2],
}

impl Camera {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Camera {
            position: [0.0, 0.0],
            zoom: 1.0,
            rotation: 0.0,
            viewport_width,
            viewport_height,
            shake_intensity: 0.0,
            shake_decay: 5.0,
            shake_offset: [0.0, 0.0],
        }
    }

    pub fn shake(&mut self, intensity: f32) {
        self.shake_intensity = self.shake_intensity.max(intensity);
    }

    pub fn set_shake_decay(&mut self, decay: f32) {
        self.shake_decay = decay;
    }

    pub fn update_shake(&mut self, dt: f32) {
        if self.shake_intensity > 0.01 {
            use rand::Rng;
            let mut rng = rand::rng();
            let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
            self.shake_offset = [
                angle.cos() * self.shake_intensity,
                angle.sin() * self.shake_intensity,
            ];
            self.shake_intensity *= (1.0 - self.shake_decay * dt).max(0.0);
        } else {
            self.shake_intensity = 0.0;
            self.shake_offset = [0.0, 0.0];
        }
    }

    pub fn effective_position(&self) -> [f32; 2] {
        [
            self.position[0] + self.shake_offset[0],
            self.position[1] + self.shake_offset[1],
        ]
    }

    pub fn get_proj_matrix(&self, device: &Device, _queue: &Queue) -> Buffer {
        let half_w = self.viewport_width / 2.0 / self.zoom;
        let half_h = self.viewport_height / 2.0 / self.zoom;

        let cos_a = self.rotation.cos();
        let sin_a = self.rotation.sin();

        let pos = self.effective_position();
        let tx = -(pos[0] * cos_a + pos[1] * sin_a) / half_w;
        let ty = (pos[0] * sin_a - pos[1] * cos_a) / half_h;

        let proj = [
            cos_a / half_w,   -sin_a / half_w,   0.0, 0.0,
            sin_a / half_h,    cos_a / half_h,   0.0, 0.0,
            0.0,               0.0,               1.0, 0.0,
            tx,                ty,                0.0, 1.0,
        ];

        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera-projection-buffer"),
            contents: bytemuck::cast_slice(&[proj]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }
}

// ──────────────────────────────────────────────
// Sprite Data (GPU-friendly)
// ──────────────────────────────────────────────

/// A single sprite vertex (position + UV, 16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct QuadVertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
}

/// Per-sprite GPU data (one instance per sprite).
///
/// Each sprite is a quad (2 triangles, 4 vertices). This struct
/// stores the per-instance data: position, size, UV rect, layer, color.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpriteInstance {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv: [f32; 4],
    pub layer: f32,
    pub color: [f32; 4],
    pub parallax: f32,
}

/// A sprite to be rendered.
#[derive(Debug, Clone, Copy)]
pub struct RenderSprite {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv: (f32, f32, f32, f32),
    pub layer: i32,
    pub color: [f32; 4],
    pub parallax: f32,
}

impl RenderSprite {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        RenderSprite {
            x, y, width, height,
            uv: (0.0, 0.0, 1.0, 1.0),
            layer: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            parallax: 1.0,
        }
    }

    pub fn with_uv(mut self, u: f32, v: f32, u_end: f32, v_end: f32) -> Self {
        self.uv = (u, v, u_end, v_end);
        self
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    pub fn with_parallax(mut self, factor: f32) -> Self {
        self.parallax = factor;
        self
    }
}

// ──────────────────────────────────────────────
// SpriteBatch — Batched Quad Rendering
// ──────────────────────────────────────────────

/// Batched sprite renderer.
///
/// Sprites are collected and uploaded to the GPU in a single draw call
/// using instanced rendering. Each sprite is a quad (2 triangles, 4 vertices).
pub struct SpriteBatch {
    /// Instance data buffer (one per sprite).
    instances: Buffer,
    /// Maximum sprites this batch can hold.
    capacity: usize,
}

impl SpriteBatch {
    /// Create a new sprite batch.
    ///
    /// Allocates GPU buffers for up to `max_sprites` instances.
    pub fn new(device: &Device, max_sprites: usize) -> Self {
        let instance_size = std::mem::size_of::<SpriteInstance>();
        let buffer_size = (instance_size * max_sprites) as u64;

        let instances = device.create_buffer(&BufferDescriptor {
            label: Some("sprite-batch-instances"),
            size: buffer_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        SpriteBatch {
            instances,
            capacity: max_sprites,
        }
    }

    /// Add a sprite to the batch.
    ///
    /// Returns the sprite index, or panics if the batch is full.
    pub fn add(&mut self, sprite: RenderSprite, sprites: &mut Vec<RenderSprite>) -> usize {
        assert!(sprites.len() < self.capacity, "SpriteBatch is full");
        sprites.push(sprite);
        sprites.len() - 1
    }

    /// Upload all sprites to the GPU.
    ///
    /// Sprites are sorted by layer (ascending) before upload so that
    /// lower layers are drawn first.
    pub fn upload(&self, queue: &Queue, sprites: &mut [RenderSprite]) {
        assert!(sprites.len() <= self.capacity, "Too many sprites for batch capacity");

        sprites.sort_by_key(|s| s.layer);

        let instances: Vec<SpriteInstance> = sprites.iter().map(|s| SpriteInstance {
            x: s.x,
            y: s.y,
            width: s.width,
            height: s.height,
            uv: [s.uv.0, s.uv.1, s.uv.2 - s.uv.0, s.uv.3 - s.uv.1],
            layer: s.layer as f32,
            color: s.color,
            parallax: s.parallax,
        }).collect();

        let data = bytemuck::cast_slice(&instances);
        queue.write_buffer(&self.instances, 0, data);
    }

    /// Draw all sprites using the given render pass.
    ///
    /// The caller is responsible for binding the quad vertex/index buffers
    /// and setting the render pipeline.
    pub fn draw_instanced(&self, pass: &mut RenderPass<'_>) {
        pass.set_vertex_buffer(1, self.instances.slice(..));
    }
}

// ──────────────────────────────────────────────
// Quad Geometry
// ──────────────────────────────────────────────

/// The quad vertex layout (position + UV, 16 bytes per vertex, 4 vertices).
pub const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex { x: -0.5, y: -0.5, u: 0.0, v: 0.0 },
    QuadVertex { x:  0.5, y: -0.5, u: 1.0, v: 0.0 },
    QuadVertex { x:  0.5, y:  0.5, u: 1.0, v: 1.0 },
    QuadVertex { x: -0.5, y:  0.5, u: 0.0, v: 1.0 },
];

/// The quad index layout (2 triangles, 6 indices).
pub const QUAD_INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3,
];

// ──────────────────────────────────────────────
// Renderer — wgpu Pipeline
// ──────────────────────────────────────────────

/// Errors that can occur when creating a renderer.
#[derive(Debug)]
pub enum RendererError {
    /// No compatible GPU adapter found.
    NoAdapter,
    /// Surface creation failed.
    CreateSurface,
    /// Surface configuration failed.
    ConfigSurface(wgpu::SurfaceError),
    /// Device request failed.
    RequestDevice(wgpu::RequestDeviceError),
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::NoAdapter => write!(f, "No compatible GPU adapter found"),
            RendererError::CreateSurface => write!(f, "Failed to create wgpu surface from window"),
            RendererError::ConfigSurface(e) => write!(f, "Surface config error: {:?}", e),
            RendererError::RequestDevice(e) => write!(f, "Device request error: {:?}", e),
        }
    }
}

impl std::error::Error for RendererError {}

/// The renderer — manages the wgpu device, surface, and rendering pipeline.
pub struct Renderer {
    /// The wgpu surface.
    pub surface: Surface<'static>,
    /// Surface configuration (width, height, format, present mode).
    pub config: SurfaceConfiguration,
    /// The wgpu device.
    pub device: Device,
    /// The wgpu queue.
    pub queue: Queue,
    /// Render pipeline.
    pub pipeline: RenderPipeline,
    /// Quad vertex buffer.
    pub quad_vertex_buffer: Buffer,
    /// Quad index buffer.
    pub quad_index_buffer: Buffer,
    /// Number of quad vertices.
    pub quad_vertex_count: u32,
    /// Number of quad indices.
    pub quad_index_count: u32,
    /// Camera uniform bind group layout.
    pub camera_bind_group_layout: BindGroupLayout,
    /// Camera bind group (recreated each frame with updated projection).
    pub camera_bind_group: BindGroup,
    /// Camera projection matrix buffer (recreated each frame).
    pub camera_buffer: Buffer,
    /// Texture bind group layout.
    pub texture_bind_group_layout: BindGroupLayout,
    /// Sprite batch for instanced quad rendering.
    pub sprite_batch: SpriteBatch,
}

impl Renderer {
     pub async fn new(window: Arc<winit::window::Window>, width: u32, height: u32) -> Result<Self, RendererError> {
        let instance = wgpu::Instance::default();

        let surface = instance
            .create_surface(window)
            .map_err(|_| RendererError::CreateSurface)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or(RendererError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor::default(),
                None,
            )
            .await
            .map_err(RendererError::RequestDevice)?;

        Self::build(surface, width, height, device, queue, &adapter)
    }

    /// Build the renderer from an existing device and queue.
    fn build(
        surface: Surface<'static>,
        width: u32,
        height: u32,
        device: Device,
        queue: Queue,
        adapter: &wgpu::Adapter,
    ) -> Result<Self, RendererError> {
        // Query the surface capabilities to find a supported format.
        let caps = surface.get_capabilities(adapter);
        let format = caps.formats.first()
            .copied()
            .unwrap_or(TextureFormat::Bgra8UnormSrgb);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Quad vertex buffer
        let quad_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("quad-vertex-buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: BufferUsages::VERTEX,
        });

        // Quad index buffer
        let quad_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("quad-index-buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: BufferUsages::INDEX,
        });

        let quad_vertex_count = QUAD_VERTICES.len() as u32;
        let quad_index_count = QUAD_INDICES.len() as u32;

        // Camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera-bgl"),
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

        // Camera buffer (4x4 matrix = 16 f32 = 64 bytes)
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("camera-buffer"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Texture bind group layout
        let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("texture-bgl"),
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

        // Sprite batch for instanced rendering
        let sprite_batch = SpriteBatch::new(&device, 1024);

        // Create the shader module (embedded WGSL)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite-renderer.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./render.wgsl").into()),
        });

        // Create the pipeline
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite-pipeline-layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite-render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as u64,
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
                    },
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteInstance>() as u64,
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: 0,
                                shader_location: 2,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: 4,
                                shader_location: 3,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: 8,
                                shader_location: 4,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: 12,
                                shader_location: 5,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: 16,
                                shader_location: 6,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: 32,
                                shader_location: 7,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: 36,
                                shader_location: 8,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: 52,
                                shader_location: 9,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Renderer {
            surface,
            config,
            device,
            queue,
            pipeline,
            quad_vertex_buffer,
            quad_index_buffer,
            quad_vertex_count,
            quad_index_count,
            camera_bind_group_layout,
            camera_bind_group,
            camera_buffer,
            texture_bind_group_layout,
            sprite_batch,
        })
    }

    /// Resize the renderer.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Render a frame.
    pub fn render(
        &mut self,
        camera: &Camera,
        sprites: &mut [RenderSprite],
        texture_view: &TextureView,
        sampler: &Sampler,
    ) {
        // Upload sprite instances to the GPU.
        self.sprite_batch.upload(&self.queue, sprites);

        // Update camera projection matrix buffer.
        self.camera_buffer = camera.get_proj_matrix(&self.device, &self.queue);

        // Recreate the camera bind group with the updated buffer.
        self.camera_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &self.camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.camera_buffer.as_entire_binding(),
            }],
        });

        // Acquire swapchain texture.
        let output = self.surface.get_current_texture()
            .expect("Failed to acquire swapchain texture");
        let view = output.texture.create_view(&TextureViewDescriptor::default());

        // Create texture bind group.
        let texture_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("texture-bind-group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("render-encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &texture_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.quad_index_buffer.slice(..), IndexFormat::Uint16);

            self.sprite_batch.draw_instanced(&mut render_pass);

            // Draw one quad per sprite instance.
            render_pass.draw_indexed(0..self.quad_index_count, 0, 0..sprites.len() as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

/// Convert visible tiles from a TileMap into RenderSprite batches.
///
/// Uses the atlas to look up UV frames for each tile index. Tiles with
/// `frame == 0` are skipped (empty/air). The `layer` parameter sets the
/// z-order for all produced sprites.
pub fn tilemap_to_sprites(
    tilemap: &crate::tilemap::TileMap,
    atlas: &crate::texture::TextureAtlas,
    cam_x: f32,
    cam_y: f32,
    view_w: f32,
    view_h: f32,
    layer: i32,
    color: [f32; 4],
) -> Vec<RenderSprite> {
    let mut sprites = Vec::new();
    let tile_size = tilemap.tile_size;

    for chunk in tilemap.visible_chunks(cam_x, cam_y, view_w, view_h) {
        let base_x = chunk.cx as f32 * crate::tilemap::CHUNK_SIZE as f32 * tile_size;
        let base_y = chunk.cy as f32 * crate::tilemap::CHUNK_SIZE as f32 * tile_size;

        for ly in 0..crate::tilemap::CHUNK_SIZE {
            for lx in 0..crate::tilemap::CHUNK_SIZE {
                let tile = chunk.get_tile(lx, ly);
                if tile.frame == 0 {
                    continue;
                }

                let frame_name = format!("tile_{}", tile.frame);
                let frame = match atlas.get_frame(&frame_name) {
                    Some(f) => f,
                    None => continue,
                };

                let world_x = base_x + lx as f32 * tile_size + tile_size * 0.5;
                let world_y = base_y + ly as f32 * tile_size + tile_size * 0.5;

                sprites.push(
                    RenderSprite::new(world_x, world_y, tile_size, tile_size)
                        .with_uv(frame.u, frame.v, frame.u + frame.du, frame.v + frame.dv)
                        .with_layer(layer)
                        .with_color(color[0], color[1], color[2], color[3]),
                );
            }
        }
    }

    sprites
}
