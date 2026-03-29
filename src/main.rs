mod canvas;
mod gpu;
mod layers;

use eframe::egui;
use layers::BlendMode;

const TOOLS: &[&str] = &[
    "Select", "Move", "Brush", "Eraser", "Fill", "Text", "Eyedropper", "Crop",
];

#[derive(Clone, Copy)]
enum ViewAction {
    ZoomIn,
    ZoomOut,
    FitToWindow,
    ZoomTo100,
}

struct LayerrdApp {
    selected_tool: usize,
    document: layers::Document,
    pipelines: gpu::GpuPipelines,
    device: wgpu::Device,
    queue: wgpu::Queue,
    // Zoom/pan state (view-only, not saved with document)
    zoom_level: f32,
    pan_offset: egui::Vec2,
    is_panning: bool,
    pending_view_action: Option<ViewAction>,
    last_viewport_size: egui::Vec2,
}

impl LayerrdApp {
    const ZOOM_MIN: f32 = 0.01;
    const ZOOM_MAX: f32 = 64.0;
    const ZOOM_STEP: f32 = 1.15;

    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let wgpu_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("wgpu backend required");

        let device = wgpu_state.device.clone();
        let queue = wgpu_state.queue.clone();

        // Initialize the canvas render resources and store in callback resources
        let target_format = wgpu_state.target_format;
        let canvas_resources = canvas::CanvasRenderResources::new(&device, target_format);
        wgpu_state
            .renderer
            .write()
            .callback_resources
            .insert(canvas_resources);

        // Initialize GPU compute pipelines
        let pipelines = gpu::GpuPipelines::new(&device);

        // Create a document with default canvas size
        let document = layers::Document::new(&device, &queue, 1920, 1080);

        Self {
            selected_tool: 0,
            document,
            pipelines,
            device,
            queue,
            zoom_level: 1.0,
            pan_offset: egui::Vec2::ZERO,
            is_panning: false,
            pending_view_action: None,
            last_viewport_size: egui::Vec2::new(1280.0, 800.0),
        }
    }

    /// Base scale that fits the canvas into the viewport with padding.
    fn fit_scale(&self, viewport_size: egui::Vec2) -> f32 {
        let canvas_w = self.document.width as f32;
        let canvas_h = self.document.height as f32;
        (viewport_size.x / canvas_w)
            .min(viewport_size.y / canvas_h)
            .min(1.0)
            * 0.9
    }

    /// Reset to fit-to-window view.
    fn fit_to_window(&mut self) {
        self.zoom_level = 1.0;
        self.pan_offset = egui::Vec2::ZERO;
    }

    /// Zoom to 100% (1 canvas pixel = 1 screen pixel), centered.
    fn zoom_to_100(&mut self, viewport_size: egui::Vec2) {
        let base = self.fit_scale(viewport_size);
        self.zoom_level = 1.0 / base;
        self.pan_offset = egui::Vec2::ZERO;
    }

    /// Zoom by a factor centered on a viewport-space cursor position.
    fn zoom_around(
        &mut self,
        factor: f32,
        center: egui::Pos2,
        viewport_rect: egui::Rect,
        viewport_size: egui::Vec2,
    ) {
        let old_zoom = self.zoom_level;
        let new_zoom = (self.zoom_level * factor).clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        let base_scale = self.fit_scale(viewport_size);

        // Cursor position in viewport UV [0,1]
        let cursor_uv = egui::vec2(
            (center.x - viewport_rect.min.x) / viewport_size.x,
            (center.y - viewport_rect.min.y) / viewport_size.y,
        );

        // Canvas-space point under cursor (in canvas pixels from canvas center)
        let old_effective = base_scale * old_zoom;
        let canvas_point = egui::vec2(
            (cursor_uv.x - 0.5) * viewport_size.x / old_effective + self.pan_offset.x,
            (cursor_uv.y - 0.5) * viewport_size.y / old_effective + self.pan_offset.y,
        );

        // Adjust pan so the same canvas point stays under cursor at new zoom
        let new_effective = base_scale * new_zoom;
        self.pan_offset = egui::vec2(
            canvas_point.x - (cursor_uv.x - 0.5) * viewport_size.x / new_effective,
            canvas_point.y - (cursor_uv.y - 0.5) * viewport_size.y / new_effective,
        );

        self.zoom_level = new_zoom;
    }

    /// Zoom by a factor centered on the viewport center.
    fn zoom_centered(&mut self, factor: f32, viewport_rect: egui::Rect, viewport_size: egui::Vec2) {
        let center = viewport_rect.center();
        self.zoom_around(factor, center, viewport_rect, viewport_size);
    }

    /// Actual zoom percentage (100% = 1 canvas pixel = 1 screen pixel).
    fn zoom_percentage(&self, viewport_size: egui::Vec2) -> f32 {
        self.fit_scale(viewport_size) * self.zoom_level * 100.0
    }
}

impl eframe::App for LayerrdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Recomposite layers if dirty (GPU compute)
        self.document
            .recomposite(&self.device, &self.queue, &self.pipelines);

        // --- Menu bar ---
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() { ui.close(); }
                    if ui.button("Open").clicked() { ui.close(); }
                    if ui.button("Save").clicked() { ui.close(); }
                    if ui.button("Save As...").clicked() { ui.close(); }
                    ui.separator();
                    if ui.button("Export").clicked() { ui.close(); }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() { ui.close(); }
                    if ui.button("Redo").clicked() { ui.close(); }
                    ui.separator();
                    if ui.button("Cut").clicked() { ui.close(); }
                    if ui.button("Copy").clicked() { ui.close(); }
                    if ui.button("Paste").clicked() { ui.close(); }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Zoom In").clicked() {
                        self.pending_view_action = Some(ViewAction::ZoomIn);
                        ui.close();
                    }
                    if ui.button("Zoom Out").clicked() {
                        self.pending_view_action = Some(ViewAction::ZoomOut);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Fit to Window").clicked() {
                        self.pending_view_action = Some(ViewAction::FitToWindow);
                        ui.close();
                    }
                    if ui.button("Zoom to 100%").clicked() {
                        self.pending_view_action = Some(ViewAction::ZoomTo100);
                        ui.close();
                    }
                });
                ui.menu_button("Image", |ui| {
                    if ui.button("Brightness/Contrast").clicked() { ui.close(); }
                    ui.separator();
                    if ui.button("Resize Canvas").clicked() { ui.close(); }
                    if ui.button("Rotate 90 CW").clicked() { ui.close(); }
                    if ui.button("Rotate 90 CCW").clicked() { ui.close(); }
                    if ui.button("Flip Horizontal").clicked() { ui.close(); }
                    if ui.button("Flip Vertical").clicked() { ui.close(); }
                });
                ui.menu_button("Layer", |ui| {
                    if ui.button("New Layer").clicked() {
                        let n = self.document.layers.len();
                        let layer = layers::Layer::new(
                            &self.device,
                            self.document.width,
                            self.document.height,
                            &format!("Layer {}", n),
                            [0, 0, 0, 0],
                        );
                        let clear = vec![0u8; (self.document.width * self.document.height * 4) as usize];
                        layer.write_data(&self.queue, self.document.width, self.document.height, &clear);
                        self.document.layers.push(layer);
                        self.document.selected_layer = n;
                        self.document.dirty = true;
                        ui.close();
                    }
                    if ui.button("Delete Layer").clicked() {
                        if self.document.layers.len() > 1 {
                            self.document.layers.remove(self.document.selected_layer);
                            self.document.selected_layer =
                                self.document.selected_layer.min(self.document.layers.len() - 1);
                            self.document.dirty = true;
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Flatten Image").clicked() { ui.close(); }
                });
            });
        });

        // --- Status bar ---
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Tool: {}", TOOLS[self.selected_tool]));
                ui.separator();
                ui.label(format!(
                    "Canvas: {} x {}",
                    self.document.width, self.document.height
                ));
                ui.separator();
                ui.label(format!(
                    "Layers: {}",
                    self.document.layers.len()
                ));
                ui.separator();
                ui.label(format!(
                    "Zoom: {:.0}%",
                    self.zoom_percentage(self.last_viewport_size)
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("GPU Accelerated");
                });
            });
        });

        // --- Left toolbar ---
        egui::SidePanel::left("toolbar")
            .exact_width(48.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    for (i, tool) in TOOLS.iter().enumerate() {
                        let label = tool.chars().next().unwrap_or('?').to_string();
                        let selected = self.selected_tool == i;
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(&label).monospace().size(16.0),
                                )
                                .min_size(egui::vec2(36.0, 36.0))
                                .selected(selected),
                            )
                            .on_hover_text(*tool)
                            .clicked()
                        {
                            self.selected_tool = i;
                        }
                    }
                });
            });

        // --- Right panel (Properties + Layers) ---
        egui::SidePanel::right("properties_panel")
            .default_width(250.0)
            .width_range(200.0..=400.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Properties for selected layer
                    egui::CollapsingHeader::new("Properties").default_open(true).show(ui, |ui| {
                        if let Some(layer) = self
                            .document
                            .layers
                            .get_mut(self.document.selected_layer)
                        {
                            ui.horizontal(|ui| {
                                ui.label("Opacity:");
                                let mut pct = layer.opacity * 100.0;
                                if ui
                                    .add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix("%"))
                                    .changed()
                                {
                                    layer.opacity = pct / 100.0;
                                    self.document.dirty = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Blend:");
                                let current = layer.blend_mode;
                                egui::ComboBox::from_id_salt("blend_mode")
                                    .selected_text(current.name())
                                    .show_ui(ui, |ui| {
                                        for &mode in BlendMode::ALL {
                                            if ui
                                                .selectable_value(
                                                    &mut layer.blend_mode,
                                                    mode,
                                                    mode.name(),
                                                )
                                                .changed()
                                            {
                                                self.document.dirty = true;
                                            }
                                        }
                                    });
                            });
                        }
                    });

                    ui.separator();

                    // Layers list
                    egui::CollapsingHeader::new("Layers").default_open(true).show(ui, |ui| {
                        let mut dirty = false;
                        for i in (0..self.document.layers.len()).rev() {
                            let layer = &mut self.document.layers[i];
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut layer.visible, "").changed() {
                                    dirty = true;
                                }
                                let selected = self.document.selected_layer == i;
                                if ui.selectable_label(selected, &layer.name).clicked() {
                                    self.document.selected_layer = i;
                                }
                            });
                        }
                        if dirty {
                            self.document.dirty = true;
                        }
                        ui.add_space(4.0);
                        if ui.button("+ New Layer").clicked() {
                            let n = self.document.layers.len();
                            let layer = layers::Layer::new(
                                &self.device,
                                self.document.width,
                                self.document.height,
                                &format!("Layer {}", n),
                                [0, 0, 0, 0],
                            );
                            let clear =
                                vec![0u8; (self.document.width * self.document.height * 4) as usize];
                            layer.write_data(
                                &self.queue,
                                self.document.width,
                                self.document.height,
                                &clear,
                            );
                            self.document.layers.push(layer);
                            self.document.selected_layer = n;
                            self.document.dirty = true;
                        }
                    });
                });
            });

        // --- Central canvas area (GPU-rendered) ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let viewport_size = rect.size();
            self.last_viewport_size = viewport_size;

            // --- Input handling ---
            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

            // Scroll wheel: zoom centered on cursor
            if response.hovered() {
                let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
                if scroll_delta.abs() > 0.1 {
                    if let Some(hover_pos) = response.hover_pos() {
                        // Scale factor proportionally: ~50 points per mouse tick
                        let ticks = scroll_delta / 50.0;
                        let factor = Self::ZOOM_STEP.powf(ticks);
                        self.zoom_around(factor, hover_pos, rect, viewport_size);
                    }
                }
            }

            // Space + left-drag OR middle mouse drag: pan
            let space_held = ui.input(|i| i.key_down(egui::Key::Space));
            self.is_panning = space_held;

            let panning = (space_held && response.dragged_by(egui::PointerButton::Primary))
                || response.dragged_by(egui::PointerButton::Middle);

            if panning {
                let drag = response.drag_delta();
                let base_scale = self.fit_scale(viewport_size);
                let effective_scale = base_scale * self.zoom_level;
                // Convert screen-pixel drag to canvas-pixel offset
                self.pan_offset.x -= drag.x / effective_scale;
                self.pan_offset.y -= drag.y / effective_scale;
            }

            // Pan cursor
            if self.is_panning {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
            }

            // Keyboard shortcuts: Ctrl+Plus, Ctrl+Minus, Ctrl+0, Ctrl+1
            let (zoom_in, zoom_out, fit, actual) = ctx.input(|i| {
                (
                    i.modifiers.command
                        && (i.key_pressed(egui::Key::Plus)
                            || i.key_pressed(egui::Key::Equals)),
                    i.modifiers.command && i.key_pressed(egui::Key::Minus),
                    i.modifiers.command && i.key_pressed(egui::Key::Num0),
                    i.modifiers.command && i.key_pressed(egui::Key::Num1),
                )
            });

            if zoom_in {
                self.zoom_centered(Self::ZOOM_STEP, rect, viewport_size);
            }
            if zoom_out {
                self.zoom_centered(1.0 / Self::ZOOM_STEP, rect, viewport_size);
            }
            if fit {
                self.fit_to_window();
            }
            if actual {
                self.zoom_to_100(viewport_size);
            }

            // Process deferred View menu actions
            if let Some(action) = self.pending_view_action.take() {
                match action {
                    ViewAction::ZoomIn => self.zoom_centered(Self::ZOOM_STEP, rect, viewport_size),
                    ViewAction::ZoomOut => {
                        self.zoom_centered(1.0 / Self::ZOOM_STEP, rect, viewport_size)
                    }
                    ViewAction::FitToWindow => self.fit_to_window(),
                    ViewAction::ZoomTo100 => self.zoom_to_100(viewport_size),
                }
            }

            // --- Compute canvas_rect_min/max from zoom/pan state ---
            let canvas_w = self.document.width as f32;
            let canvas_h = self.document.height as f32;
            let base_scale = self.fit_scale(viewport_size);
            let effective_scale = base_scale * self.zoom_level;
            let display_w = canvas_w * effective_scale;
            let display_h = canvas_h * effective_scale;

            let pan_uv_x = self.pan_offset.x * effective_scale / viewport_size.x;
            let pan_uv_y = self.pan_offset.y * effective_scale / viewport_size.y;

            let cx = 0.5 - pan_uv_x;
            let cy = 0.5 - pan_uv_y;
            let half_w = display_w / viewport_size.x * 0.5;
            let half_h = display_h / viewport_size.y * 0.5;

            let callback = canvas::CanvasPaintCallback {
                canvas_texture_view: self.document.composite_view.clone(),
                canvas_rect_min: [cx - half_w, cy - half_h],
                canvas_rect_max: [cx + half_w, cy + half_h],
                viewport_size: [viewport_size.x, viewport_size.y],
            };

            ui.painter()
                .add(egui_wgpu::Callback::new_paint_callback(rect, callback));
        });
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Layerrd",
        options,
        Box::new(|cc| Ok(Box::new(LayerrdApp::new(cc)))),
    )
}
