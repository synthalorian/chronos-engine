#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Texture atlas loading and GPU texture management.

use std::collections::HashMap;
use wgpu::{
    Device, Extent3d, ImageCopyTexture, ImageDataLayout, Queue, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

#[derive(Debug, Clone, Copy)]
pub struct AtlasFrame {
    pub u: f32,
    pub v: f32,
    pub du: f32,
    pub dv: f32,
    pub pixel_w: u32,
    pub pixel_h: u32,
}

pub struct TextureAtlas {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub width: u32,
    pub height: u32,
    frames: HashMap<String, AtlasFrame>,
}

impl TextureAtlas {
    pub fn from_rgba(
        device: &Device,
        queue: &Queue,
        pixels: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            pixels,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        TextureAtlas {
            texture,
            texture_view,
            width,
            height,
            frames: HashMap::new(),
        }
    }

    pub fn from_path(device: &Device, queue: &Queue, path: &str) -> Result<Self, String> {
        let img = image::ImageReader::open(path)
            .map_err(|e| format!("Failed to open image {}: {}", path, e))?
            .decode()
            .map_err(|e| format!("Failed to decode image {}: {}", path, e))?;

        let rgba = img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        Ok(Self::from_rgba(
            device,
            queue,
            &rgba,
            width,
            height,
            Some(path),
        ))
    }

    pub fn define_frame(&mut self, name: &str, x: u32, y: u32, w: u32, h: u32) {
        let u = x as f32 / self.width as f32;
        let v = y as f32 / self.height as f32;
        let du = w as f32 / self.width as f32;
        let dv = h as f32 / self.height as f32;

        self.frames.insert(
            name.to_string(),
            AtlasFrame {
                u,
                v,
                du,
                dv,
                pixel_w: w,
                pixel_h: h,
            },
        );
    }

    pub fn define_grid(&mut self, tile_w: u32, tile_h: u32, columns: u32, rows: u32) {
        for row in 0..rows {
            for col in 0..columns {
                let idx = row * columns + col;
                let name = format!("tile_{}", idx);
                let x = col * tile_w;
                let y = row * tile_h;
                self.define_frame(&name, x, y, tile_w, tile_h);
            }
        }
    }

    pub fn get_frame(&self, name: &str) -> Option<&AtlasFrame> {
        self.frames.get(name)
    }

    pub fn white_pixel(device: &Device, queue: &Queue) -> Self {
        let pixels = [255u8, 255, 255, 255];
        Self::from_rgba(device, queue, &pixels, 1, 1, Some("white-pixel"))
    }
}

pub struct FpsCounter {
    frame_times: Vec<std::time::Instant>,
    max_samples: usize,
}

impl FpsCounter {
    pub fn new(max_samples: usize) -> Self {
        FpsCounter {
            frame_times: Vec::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn tick(&mut self) {
        let now = std::time::Instant::now();
        self.frame_times.push(now);
        if self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }
    }

    pub fn fps(&self) -> f64 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        #[allow(clippy::unwrap_used)]
        let first = self.frame_times.first().unwrap();
        let last = self
            .frame_times
            .last()
            .expect("frame_times should have at least one entry after first frame");
        let elapsed = last.duration_since(*first).as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        (self.frame_times.len() - 1) as f64 / elapsed
    }

    pub fn frame_time_ms(&self) -> f64 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let len = self.frame_times.len();
        let elapsed = self.frame_times[len - 1]
            .duration_since(self.frame_times[len - 2])
            .as_secs_f64();
        elapsed * 1000.0
    }
}
