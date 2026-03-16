mod canvas;
mod gpu;
mod layers;

use eframe::egui;
use layers::BlendMode;

const TOOLS: &[&str] = &[
    "Select", "Move", "Brush", "Eraser", "Fill", "Text", "Eyedropper", "Crop",
];

struct LayerrdApp {
    selected_tool: usize,
    document: layers::Document,
    pipelines: gpu::GpuPipelines,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl LayerrdApp {
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
        }
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
                    if ui.button("Zoom In").clicked() { ui.close(); }
                    if ui.button("Zoom Out").clicked() { ui.close(); }
                    if ui.button("Fit to Window").clicked() { ui.close(); }
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

            // Calculate canvas position centered in the panel
            let canvas_w = self.document.width as f32;
            let canvas_h = self.document.height as f32;

            // Fit canvas into viewport with some padding
            let scale = (viewport_size.x / canvas_w)
                .min(viewport_size.y / canvas_h)
                .min(1.0)
                * 0.9;

            let display_w = canvas_w * scale;
            let display_h = canvas_h * scale;

            // Canvas rect in UV space [0,1] relative to the panel
            let cx = 0.5;
            let cy = 0.5;
            let half_w = display_w / viewport_size.x * 0.5;
            let half_h = display_h / viewport_size.y * 0.5;

            let callback = canvas::CanvasPaintCallback {
                canvas_texture_view: self.document.composite_view.clone(),
                canvas_rect_min: [cx - half_w, cy - half_h],
                canvas_rect_max: [cx + half_w, cy + half_h],
                viewport_size: [viewport_size.x, viewport_size.y],
            };

            // Allocate the space and add the paint callback
            ui.allocate_rect(rect, egui::Sense::click_and_drag());
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
