use std::sync::Arc;

/// Blend mode for layer compositing (maps to shader blend_mode uniform).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
}

impl BlendMode {
    pub const ALL: &[BlendMode] = &[
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
        }
    }
}

/// A single image layer with GPU-backed texture data.
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub texture: wgpu::Texture,
    pub view: Arc<wgpu::TextureView>,
}

impl Layer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, name: &str, _fill: [u8; 4]) -> Self {
        let texture = crate::gpu::create_layer_texture(device, width, height, name);
        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));

        Self {
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            texture,
            view,
        }
    }

    /// Write RGBA pixel data to this layer's texture.
    pub fn write_data(&self, queue: &wgpu::Queue, width: u32, height: u32, data: &[u8]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
    }
}

/// The document: canvas dimensions + a stack of layers.
pub struct Document {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub selected_layer: usize,
    /// The composited result texture, updated by the GPU compositor.
    pub composite_texture: wgpu::Texture,
    pub composite_view: Arc<wgpu::TextureView>,
    /// Scratch texture for ping-pong compositing.
    scratch_texture: wgpu::Texture,
    scratch_view: wgpu::TextureView,
    /// Whether the composite needs to be recomputed.
    pub dirty: bool,
}

impl Document {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let composite_texture = crate::gpu::create_layer_texture(device, width, height, "composite");
        let composite_view = Arc::new(
            composite_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let scratch_texture = crate::gpu::create_layer_texture(device, width, height, "scratch");
        let scratch_view =
            scratch_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create a white background layer
        let bg = Layer::new(device, width, height, "Background", [255, 255, 255, 255]);
        let bg_data = vec![255u8; (width * height * 4) as usize];
        bg.write_data(queue, width, height, &bg_data);

        // Create an empty transparent layer
        let layer1 = Layer::new(device, width, height, "Layer 1", [0, 0, 0, 0]);
        // Write a colored rectangle in the center as a demo
        let mut data = vec![0u8; (width * height * 4) as usize];
        let cx = width / 2;
        let cy = height / 2;
        let rect_w = width / 4;
        let rect_h = height / 4;
        for y in (cy - rect_h / 2)..(cy + rect_h / 2) {
            for x in (cx - rect_w / 2)..(cx + rect_w / 2) {
                let idx = ((y * width + x) * 4) as usize;
                data[idx] = 70;     // R
                data[idx + 1] = 130; // G
                data[idx + 2] = 220; // B
                data[idx + 3] = 200; // A
            }
        }
        layer1.write_data(queue, width, height, &data);

        Self {
            width,
            height,
            layers: vec![bg, layer1],
            selected_layer: 1,
            composite_texture,
            composite_view,
            scratch_texture,
            scratch_view,
            dirty: true,
        }
    }

    /// Recomposite all visible layers using GPU compute shaders.
    /// Uses ping-pong between composite and scratch textures.
    pub fn recomposite(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipelines: &crate::gpu::GpuPipelines,
    ) {
        if !self.dirty {
            return;
        }

        let visible_layers: Vec<usize> = self
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.visible)
            .map(|(i, _)| i)
            .collect();

        if visible_layers.is_empty() {
            // Clear composite to transparent
            let clear_data = vec![0u8; (self.width * self.height * 4) as usize];
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.composite_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &clear_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.width),
                    rows_per_image: Some(self.height),
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
            self.dirty = false;
            return;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("composite_encoder"),
        });

        // Copy first visible layer to composite as the base
        let first = &self.layers[visible_layers[0]];
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &first.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.composite_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // Composite remaining visible layers using ping-pong
        // composite holds current result, scratch is temp output
        let mut result_in_composite = true;

        for &layer_idx in &visible_layers[1..] {
            let layer = &self.layers[layer_idx];
            let (dst_view, out_view) = if result_in_composite {
                (&*self.composite_view, &self.scratch_view)
            } else {
                (&self.scratch_view, &*self.composite_view)
            };

            pipelines.composite(
                device,
                &mut encoder,
                &layer.view,
                dst_view,
                out_view,
                self.width,
                self.height,
                layer.opacity,
                layer.blend_mode as u32,
            );

            result_in_composite = !result_in_composite;
        }

        // If result ended up in scratch, copy back to composite
        if !result_in_composite {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.scratch_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.composite_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        queue.submit(std::iter::once(encoder.finish()));
        self.dirty = false;
    }
}
