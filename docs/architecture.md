# Layerrd - System Architecture

## Overview

Layerrd is a GPU-accelerated image editor built in Rust using **eframe/egui** for the UI and **wgpu** for all GPU operations (rendering and compute). The application follows a Photoshop-like layer-based editing model where each layer is a GPU texture and all compositing happens on the GPU via compute shaders.

## Module Structure

```
src/
  main.rs          # Application entry point, UI layout, event handling
  layers.rs        # Document model: Layer, BlendMode, Document
  gpu.rs           # GPU compute pipelines (composite, brightness/contrast)
  canvas.rs        # Canvas rendering via egui_wgpu paint callbacks
  shaders/
    composite.wgsl           # Layer compositing compute shader (6 blend modes)
    brightness_contrast.wgsl # Brightness/contrast adjustment compute shader
    checkerboard.wgsl        # Canvas display render shader (checkerboard + image)
```

## Core Components

### `LayerrdApp` (main.rs)

The top-level application struct implementing `eframe::App`. Holds:
- `document: Document` -- the current image document
- `pipelines: GpuPipelines` -- GPU compute pipelines
- `device` / `queue` -- wgpu device and command queue
- `selected_tool: usize` -- index into the `TOOLS` array

The `update()` method drives the frame loop:
1. Recomposites layers if dirty (GPU compute)
2. Renders egui panels: menu bar, status bar, left toolbar, right properties/layers panel, central canvas

### `Document` / `Layer` (layers.rs)

**`Layer`** -- A single image layer backed by a `wgpu::Texture` (Rgba8Unorm). Properties: name, visible, opacity (0.0-1.0), blend mode. The texture view is wrapped in `Arc<wgpu::TextureView>` for shared access across render callbacks.

**`BlendMode`** -- Enum mapping to shader uniform values: Normal (0), Multiply (1), Screen (2), Overlay (3), Darken (4), Lighten (5).

**`Document`** -- Owns the layer stack, canvas dimensions, and compositing textures:
- `layers: Vec<Layer>` -- ordered bottom-to-top
- `composite_texture` / `composite_view` -- final composited result (Arc-shared with the canvas renderer)
- `scratch_texture` / `scratch_view` -- temporary texture for ping-pong compositing
- `dirty: bool` -- set to true when layers change; triggers recomposite on next frame

### `GpuPipelines` (gpu.rs)

Holds wgpu compute pipelines and their bind group layouts:

**Composite pipeline** -- Binds: src texture (layer), dst texture (accumulated result), output storage texture, uniform params (width, height, opacity, blend_mode). Dispatches 16x16 workgroups.

**Brightness/contrast pipeline** -- Binds: src texture, output storage texture, uniform params (width, height, brightness, contrast). Not yet wired into the UI.

Helper: `create_layer_texture()` creates Rgba8Unorm textures with TEXTURE_BINDING | STORAGE_BINDING | COPY_DST | COPY_SRC usage flags.

### `CanvasRenderResources` / `CanvasPaintCallback` (canvas.rs)

Integrates custom wgpu rendering into egui via `egui_wgpu::CallbackTrait`:

**`CanvasRenderResources`** -- A render pipeline stored in egui's `CallbackResources`. Uses the checkerboard shader to draw a full-screen triangle, compositing the canvas image over a transparency checkerboard pattern.

**`CanvasPaintCallback`** -- Created each frame with the current composite texture view and canvas positioning (UV rect). In `prepare()`, it creates a per-frame bind group and params buffer. In `paint()`, it draws a single full-screen triangle (3 vertices, no vertex buffer).

## Data Flow

```
Layer textures (GPU)
       |
       v
  Document::recomposite()
  [composite compute shader, ping-pong between composite & scratch textures]
       |
       v
  composite_texture (Arc<TextureView>)
       |
       v
  CanvasPaintCallback
  [checkerboard render shader, full-screen triangle]
       |
       v
  egui viewport
```

### Compositing Algorithm

1. Copy the first visible layer directly to the composite texture
2. For each subsequent visible layer, run the composite compute shader:
   - Reads: layer texture (src) + current result (dst)
   - Writes: the other texture (ping-pong between composite and scratch)
   - Applies blend mode and opacity via alpha-over compositing
3. If the final result is in the scratch texture, copy it back to composite
4. Submit the command buffer; clear dirty flag

## UI Layout

```
+-----------------------------------------------------------+
| File | Edit | View | Image | Layer          (menu bar)    |
+------+--------------------------------------------+-------+
|  S   |                                            | Props |
|  M   |                                            |  ...  |
|  B   |         Central Canvas                     |-------|
|  E   |      (GPU-rendered, checkerboard bg)       | Layers|
|  F   |                                            |  ...  |
|  T   |                                            | [+New]|
|  D   |                                            |       |
|  C   |                                            |       |
+------+--------------------------------------------+-------+
| Tool: Brush | Canvas: 1920x1080 | Layers: 2 | GPU Accel  |
+-----------------------------------------------------------+
```

- **Left toolbar** (48px fixed): 8 tool buttons (Select, Move, Brush, Eraser, Fill, Text, Eyedropper, Crop)
- **Right panel** (200-400px): Properties (opacity slider, blend mode combo) + Layers list with visibility checkboxes
- **Central panel**: GPU-rendered canvas with auto-fit scaling and checkerboard transparency

## Dependencies

| Crate | Purpose |
|-------|---------|
| `eframe` 0.33 (wgpu feature) | Application framework, egui integration, window management |
| `egui-wgpu` 0.33 | Custom wgpu paint callbacks within egui |
| `wgpu` 27 | GPU compute and render pipelines |
| `bytemuck` 1 | Safe casting of structs to shader-compatible byte slices |
| `env_logger` / `log` | Logging |

## Key Design Decisions

- **All pixel data lives on the GPU** -- layers are wgpu textures, not CPU-side buffers. This avoids expensive GPU<->CPU transfers during editing.
- **Ping-pong compositing** -- uses two textures (composite + scratch) to avoid read-write hazards in compute shaders.
- **No sampler** -- all texture reads use `textureLoad` (integer coords), avoiding sampler allocation and filtering overhead.
- **Arc-shared texture views** -- `composite_view` is `Arc<wgpu::TextureView>` so the canvas paint callback (which runs in egui's render pass) can reference it without lifetime issues.
- **Dirty flag** -- compositing is skipped when layers haven't changed, avoiding redundant GPU work.
