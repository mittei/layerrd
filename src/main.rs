use eframe::egui;

const TOOLS: &[&str] = &[
    "Select", "Move", "Brush", "Eraser", "Fill", "Text", "Eyedropper", "Crop",
];

struct LayerrdApp {
    selected_tool: usize,
    layers: Vec<LayerEntry>,
    zoom: f32,
    opacity: f32,
    blend_mode: usize,
}

struct LayerEntry {
    name: String,
    visible: bool,
}

impl LayerrdApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            selected_tool: 0,
            layers: vec![
                LayerEntry { name: "Background".into(), visible: true },
                LayerEntry { name: "Layer 1".into(), visible: true },
                LayerEntry { name: "Layer 2".into(), visible: false },
            ],
            zoom: 100.0,
            opacity: 100.0,
            blend_mode: 0,
        }
    }
}

const BLEND_MODES: &[&str] = &[
    "Normal", "Multiply", "Screen", "Overlay", "Darken", "Lighten",
];

impl eframe::App for LayerrdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                        self.zoom = (self.zoom + 25.0).min(1600.0);
                        ui.close();
                    }
                    if ui.button("Zoom Out").clicked() {
                        self.zoom = (self.zoom - 25.0).max(25.0);
                        ui.close();
                    }
                    if ui.button("Fit to Window").clicked() {
                        self.zoom = 100.0;
                        ui.close();
                    }
                });
                ui.menu_button("Image", |ui| {
                    if ui.button("Resize Canvas").clicked() { ui.close(); }
                    if ui.button("Crop to Selection").clicked() { ui.close(); }
                    ui.separator();
                    if ui.button("Rotate 90 CW").clicked() { ui.close(); }
                    if ui.button("Rotate 90 CCW").clicked() { ui.close(); }
                    if ui.button("Flip Horizontal").clicked() { ui.close(); }
                    if ui.button("Flip Vertical").clicked() { ui.close(); }
                });
                ui.menu_button("Layer", |ui| {
                    if ui.button("New Layer").clicked() { ui.close(); }
                    if ui.button("Duplicate Layer").clicked() { ui.close(); }
                    if ui.button("Delete Layer").clicked() { ui.close(); }
                    ui.separator();
                    if ui.button("Merge Down").clicked() { ui.close(); }
                    if ui.button("Flatten Image").clicked() { ui.close(); }
                });
            });
        });

        // --- Status bar ---
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Tool: {}", TOOLS[self.selected_tool]));
                ui.separator();
                ui.label(format!("Zoom: {:.0}%", self.zoom));
                ui.separator();
                ui.label("Canvas: 1920 x 1080");
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
                            .add(egui::Button::new(
                                egui::RichText::new(&label).monospace().size(16.0),
                            ).min_size(egui::vec2(36.0, 36.0)).selected(selected))
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
                    // Properties section
                    ui.collapsing("Properties", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Opacity:");
                            ui.add(egui::Slider::new(&mut self.opacity, 0.0..=100.0).suffix("%"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Blend:");
                            egui::ComboBox::from_id_salt("blend_mode")
                                .selected_text(BLEND_MODES[self.blend_mode])
                                .show_ui(ui, |ui| {
                                    for (i, mode) in BLEND_MODES.iter().enumerate() {
                                        ui.selectable_value(&mut self.blend_mode, i, *mode);
                                    }
                                });
                        });
                    });

                    ui.separator();

                    // Layers section
                    ui.collapsing("Layers", |ui| {
                        for layer in &mut self.layers {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut layer.visible, "");
                                ui.label(&layer.name);
                            });
                        }
                        ui.add_space(4.0);
                        if ui.button("+ New Layer").clicked() {
                            let n = self.layers.len();
                            self.layers.push(LayerEntry {
                                name: format!("Layer {}", n),
                                visible: true,
                            });
                        }
                    });
                });
            });

        // --- Central canvas area ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let rect = ui.max_rect();

            // Draw checkerboard background
            let painter = ui.painter_at(rect);
            let tile = 16.0;
            let cols = (available.x / tile).ceil() as i32;
            let rows = (available.y / tile).ceil() as i32;
            let light = egui::Color32::from_gray(200);
            let dark = egui::Color32::from_gray(160);

            for row in 0..rows {
                for col in 0..cols {
                    let color = if (row + col) % 2 == 0 { light } else { dark };
                    let pos = rect.min + egui::vec2(col as f32 * tile, row as f32 * tile);
                    let tile_rect = egui::Rect::from_min_size(pos, egui::vec2(tile, tile));
                    painter.rect_filled(tile_rect, 0.0, color);
                }
            }

            // Center label
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Canvas Area",
                egui::FontId::proportional(32.0),
                egui::Color32::from_rgba_premultiplied(80, 80, 80, 180),
            );
        });
    }
}

fn main() -> eframe::Result {
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
