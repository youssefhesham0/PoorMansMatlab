use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, Polygon};
use egui_snarl::{InPin, OutPin, Snarl};
use egui_snarl::ui::{PinInfo, SnarlViewer, SnarlStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use wasmtime::*;
use serde::{Serialize, Deserialize};

// --- LOGIC STUDIO (V3) STRUCTS ---
#[derive(Clone, Serialize, Deserialize)]
struct DumbNode { name: String, inputs: Vec<String>, outputs: Vec<String> }
struct DumbViewer;
impl SnarlViewer<DumbNode> for DumbViewer {
    fn title(&mut self, node: &DumbNode) -> String { node.name.clone() }
    fn inputs(&mut self, node: &DumbNode) -> usize { node.inputs.len() }
    fn outputs(&mut self, node: &DumbNode) -> usize { node.outputs.len() }
    fn show_input(&mut self, pin: &InPin, ui: &mut egui::Ui, _scale: f32, snarl: &mut Snarl<DumbNode>) -> PinInfo {
        ui.label(&snarl[pin.id.node].inputs[pin.id.input]); PinInfo::circle().with_fill(egui::Color32::GREEN)
    }
    fn show_output(&mut self, pin: &OutPin, ui: &mut egui::Ui, _scale: f32, snarl: &mut Snarl<DumbNode>) -> PinInfo {
        ui.label(&snarl[pin.id.node].outputs[pin.id.output]); PinInfo::circle().with_fill(egui::Color32::RED)
    }
    fn has_node_menu(&mut self, _node: &DumbNode) -> bool { true }
    fn show_node_menu(&mut self, node: egui_snarl::NodeId, _inputs: &[InPin], _outputs: &[OutPin], ui: &mut egui::Ui, _scale: f32, snarl: &mut Snarl<DumbNode>) {
        if ui.button("🗑 Delete Node").clicked() { snarl.remove_node(node); ui.close_menu(); }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]), ..Default::default() };
    eframe::run_native("Poor Man's MATLAB - Universal Shell", options, Box::new(|_cc| Ok(Box::new(HostEngineApp::default()))))
}

struct HostEngineApp {
    loaded_modules: Vec<String>,
    selected_module: Option<String>,
    user_inputs: HashMap<String, String>,
    ui_blueprint: Option<serde_json::Value>,
    status: String,
    snarl: Snarl<DumbNode>,
    active_tab_id: String,
}

impl Default for HostEngineApp {
    fn default() -> Self {
        Self { loaded_modules: Vec::new(), selected_module: None, user_inputs: HashMap::new(), ui_blueprint: None, status: "Ready.".to_string(), snarl: Snarl::new(), active_tab_id: String::new() }
    }
}

impl eframe::App for HostEngineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut trigger_wasm = false;

        egui::SidePanel::left("sidebar").show(ctx, |ui| {
            ui.heading("Cartridges");
            if ui.button("Scan Modules").clicked() { self.scan_modules(); }
            ui.separator();
            
            let modules = self.loaded_modules.clone();
            for mod_name in &modules {
                if ui.selectable_label(self.selected_module.as_ref() == Some(mod_name), mod_name).clicked() {
                    self.selected_module = Some(mod_name.clone());
                    self.user_inputs.clear(); self.snarl = Snarl::new(); self.active_tab_id.clear(); trigger_wasm = true; 
                }
            }
            ui.add_space(20.0); ui.label(egui::RichText::new(&self.status).color(egui::Color32::GRAY));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_module.is_none() { ui.heading("Load a module to begin."); return; }

            if let Some(blueprint) = self.ui_blueprint.clone() {
                
                // --- THE MORPHING DECISION ENGINE ---
                // Default to "dashboard" if older code doesn't explicitly send a type
                let layout_kind = blueprint.get("type").and_then(|v| v.as_str()).unwrap_or("dashboard");

                // =========================================================
                // SHAPE 1: THE DASHBOARD (Integration/Physics Module)
                // =========================================================
                if layout_kind == "dashboard" {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.heading("Module Parameters");
                            ui.separator();
                            
                            // Check for either "inputs" or "controls" based on the module's generation
                            let controls = blueprint.get("inputs").or_else(|| blueprint.get("controls")).and_then(|c| c.as_array());
                            
                            if let Some(controls) = controls {
                                for ctrl in controls {
                                    let id = ctrl["id"].as_str().unwrap();
                                    let label = ctrl.get("label").or_else(|| ctrl.get("name")).and_then(|v| v.as_str()).unwrap_or(id);
                                    let kind = ctrl["type"].as_str().unwrap();
                                    
                                    let val = self.user_inputs.entry(id.to_string()).or_insert_with(|| ctrl.get("default").and_then(|d| d.as_str()).unwrap_or("0").to_string());

                                    ui.horizontal(|ui| {
                                        ui.label(label);
                                        if kind == "text" { ui.text_edit_singleline(val); } 
                                        else if kind == "slider" {
                                            let mut num = val.parse::<f64>().unwrap_or(0.0);
                                            let min = ctrl.get("min").and_then(|m| m.as_f64()).unwrap_or(0.0);
                                            let max = ctrl.get("max").and_then(|m| m.as_f64()).unwrap_or(100.0);
                                            if ui.add(egui::Slider::new(&mut num, min..=max)).changed() { *val = num.to_string(); }
                                        }
                                        else if kind == "radio" {
                                            if let Some(options) = ctrl.get("options").and_then(|o| o.as_array()) {
                                                for opt in options {
                                                    let opt_str = opt.as_str().unwrap();
                                                    ui.radio_value(val, opt_str.to_string(), opt_str);
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                            
                            ui.add_space(10.0);
                            if ui.button("⚡ Execute").clicked() { trigger_wasm = true; }
                            ui.separator();

                            ui.heading("Results");
                            if let Some(outputs) = blueprint.get("outputs").and_then(|o| o.as_array()) {
                                for out in outputs {
                                    let kind = out["type"].as_str().unwrap();
                                    
                                    if kind == "text" {
                                        let text = out["value"].as_str().unwrap();
                                        let color = match out.get("color").and_then(|c| c.as_str()) {
                                            Some("green") => egui::Color32::GREEN, Some("red") => egui::Color32::RED, Some("blue") => egui::Color32::LIGHT_BLUE, _ => egui::Color32::WHITE,
                                        };
                                        ui.colored_label(color, egui::RichText::new(text).heading());
                                    } 
                                    else if kind == "plot" {
                                        let title = out["title"].as_str().unwrap();
                                        Plot::new(title).height(250.0).allow_scroll(false).allow_zoom(true).allow_drag(true).legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
                                            .show(ui, |plot_ui| {
                                            
                                            if let Some(series) = out.get("series").and_then(|s| s.as_array()) {
                                                for s in series {
                                                    let s_name = s["name"].as_str().unwrap();
                                                    let s_type = s["type"].as_str().unwrap();
                                                    
                                                    let color = match s["color"].as_str().unwrap() {
                                                        "blue" => egui::Color32::from_rgba_unmultiplied(50, 150, 250, 120),
                                                        "yellow" => egui::Color32::from_rgba_unmultiplied(250, 250, 50, 150),
                                                        "red" => egui::Color32::from_rgba_unmultiplied(250, 50, 50, 150),
                                                        "white" => egui::Color32::WHITE,
                                                        _ => egui::Color32::GRAY,
                                                    };

                                                    if s_type == "line" {
                                                        if let Some(data) = s.get("data").and_then(|d| d.as_array()) {
                                                            let pts: PlotPoints = data.iter().map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()]).collect();
                                                            plot_ui.line(Line::new(pts).color(color).width(2.0).name(s_name));
                                                        }
                                                    } else if s_type == "polygon" {
                                                        if let Some(data) = s.get("data").and_then(|d| d.as_array()) {
                                                            for poly_data in data.iter() {
                                                                if let Some(pts_arr) = poly_data.as_array() {
                                                                    let pts: PlotPoints = pts_arr.iter().map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()]).collect();
                                                                    plot_ui.polygon(egui_plot::Polygon::new(pts).fill_color(color).stroke(egui::Stroke::new(1.0, color)).name(s_name));
                                                                }
                                                            }
                                                        }
                                                    } else if s_type == "outlines" {
                                                        if let Some(data) = s.get("data").and_then(|d| d.as_array()) {
                                                            for poly_data in data.iter() {
                                                                if let Some(pts_arr) = poly_data.as_array() {
                                                                    let pts: PlotPoints = pts_arr.iter().map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()]).collect();
                                                                    let solid_color = match s["color"].as_str().unwrap() {
                                                                        "blue" => egui::Color32::from_rgb(50, 150, 250), "yellow" => egui::Color32::from_rgb(250, 250, 50), "red" => egui::Color32::from_rgb(250, 50, 50), _ => egui::Color32::WHITE,
                                                                    };
                                                                    plot_ui.line(Line::new(pts).color(solid_color).width(1.5).name("")); 
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                        });
                }
                // =========================================================
                // SHAPE 2: THE TABBED STUDIO (Logic Module & Future MANET)
                // =========================================================
                else if layout_kind == "tabbed_studio" {
                    if let Some(tabs) = blueprint.get("tabs").and_then(|t| t.as_array()) {
                        ui.horizontal(|ui| {
                            for tab in tabs {
                                let id = tab["id"].as_str().unwrap_or("");
                                let title = tab["title"].as_str().unwrap_or(id);
                                ui.selectable_value(&mut self.active_tab_id, id.to_string(), title);
                            }
                        });
                        ui.separator();

                        let active_tab_data = tabs.iter().find(|t| t["id"].as_str().unwrap_or("") == self.active_tab_id).unwrap_or(&tabs[0]);
                        if self.active_tab_id.is_empty() { self.active_tab_id = active_tab_data["id"].as_str().unwrap_or("").to_string(); }
                        let tab_kind = active_tab_data["type"].as_str().unwrap_or("");
                        
                        if tab_kind == "node_graph" {
                            let mut palette: Vec<DumbNode> = vec![];
                            if let Some(nodes) = active_tab_data.get("palette").and_then(|n| n.as_array()) { palette = nodes.iter().filter_map(|n| serde_json::from_value(n.clone()).ok()).collect(); }
                            egui::TopBottomPanel::bottom("slim_tray").resizable(false).show_inside(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Library:");
                                    ui.push_id("tray_scroll", |ui| { egui::ScrollArea::horizontal().show(ui, |ui| { ui.horizontal(|ui| { for node in palette { if ui.button(&node.name).clicked() { self.snarl.insert_node(egui::pos2(0.0, 0.0), node); } } }); }); });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.button("⚡ COMPILE GRAPH").clicked() { trigger_wasm = true; } });
                                });
                            });
                            egui::CentralPanel::default().show_inside(ui, |ui| { self.snarl.show(&mut DumbViewer, &SnarlStyle::new(), "snarl_canvas", ui); });
                        } 
                        else if tab_kind == "text" {
                            let text = active_tab_data["value"].as_str().unwrap_or("");
                            let mut display_text = text.to_string();
                            egui::ScrollArea::vertical().show(ui, |ui| { ui.add(egui::TextEdit::multiline(&mut display_text).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(30)); });
                        }
                        else if tab_kind == "state_diagram" {
                            if let (Some(states), Some(transitions)) = (active_tab_data.get("states").and_then(|s| s.as_array()), active_tab_data.get("transitions").and_then(|t| t.as_array())) {
                                egui::ScrollArea::both().show(ui, |ui| {
                                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(800.0, 600.0), egui::Sense::hover());
                                    let painter = ui.painter_at(rect);
                                    let center = rect.center();
                                    let radius = 150.0;
                                    let mut state_positions = std::collections::HashMap::new();
                                    for (i, state_val) in states.iter().enumerate() {
                                        let state_str = state_val.as_str().unwrap();
                                        let angle = (i as f32 / states.len() as f32) * std::f32::consts::TAU;
                                        let pos = center + egui::vec2(angle.cos() * radius, angle.sin() * radius);
                                        state_positions.insert(state_str, pos);
                                        painter.circle_filled(pos, 40.0, egui::Color32::from_gray(40));
                                        painter.circle_stroke(pos, 40.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE));
                                        painter.text(pos, egui::Align2::CENTER_CENTER, state_str, egui::FontId::proportional(24.0), egui::Color32::WHITE);
                                    }
                                    for t in transitions {
                                        let from = t["from"].as_str().unwrap(); let to = t["to"].as_str().unwrap(); let label = t["label"].as_str().unwrap_or("");
                                        if let (Some(&p1), Some(&p2)) = (state_positions.get(from), state_positions.get(to)) {
                                            if from == to {
                                                let loop_center = p1 + egui::vec2(0.0, -60.0);
                                                painter.circle_stroke(loop_center, 20.0, egui::Stroke::new(2.0, egui::Color32::YELLOW));
                                                painter.text(loop_center + egui::vec2(0.0, -30.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), egui::Color32::LIGHT_GREEN);
                                            } else {
                                                let dir = (p2 - p1).normalized();
                                                let start_edge = p1 + dir * 40.0; let end_edge = p2 - dir * 40.0;
                                                painter.arrow(start_edge, end_edge - start_edge, egui::Stroke::new(2.0, egui::Color32::YELLOW));
                                                let mid_point = start_edge + (end_edge - start_edge) * 0.5;
                                                painter.text(mid_point + egui::vec2(0.0, -15.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), egui::Color32::LIGHT_GREEN);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            } else {
                if ui.button("⚡ Initialize Module").clicked() { trigger_wasm = true; }
            }
        }); 

        if trigger_wasm { self.fire_wasm(); }
    }
}

impl HostEngineApp {
    fn scan_modules(&mut self) {
        let folder = Path::new("modules");
        if !folder.exists() { let _ = fs::create_dir(folder); return; }
        self.loaded_modules.clear();
        if let Ok(entries) = fs::read_dir(folder) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("wasm") {
                    self.loaded_modules.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        self.status = format!("Found {} modules", self.loaded_modules.len());
    }

    fn fire_wasm(&mut self) {
        let mod_name = self.selected_module.as_ref().unwrap();
        let payload = serde_json::json!({ "inputs": self.user_inputs, "graph": &self.snarl });
        let json_input = serde_json::to_string(&payload).unwrap();
        
        let engine = Engine::default();
        let path = Path::new("modules").join(mod_name);
        
        if let Ok(module) = Module::from_file(&engine, &path) {
            let mut store = Store::new(&engine, ());
            if let Ok(instance) = Instance::new(&mut store, &module, &[]) {
                if let (Some(memory), Ok(get_ptr), Ok(execute)) = (
                    instance.get_memory(&mut store, "memory"),
                    instance.get_typed_func::<(), i32>(&mut store, "get_buffer_ptr"),
                    instance.get_typed_func::<i32, i32>(&mut store, "execute")
                ) {
                    if let Ok(ptr) = get_ptr.call(&mut store, ()) {
                        let input_bytes = json_input.as_bytes();
                        let _ = memory.write(&mut store, ptr as usize, input_bytes);
                        
                        if let Ok(out_len) = execute.call(&mut store, input_bytes.len() as i32) {
                            let mut out_bytes = vec![0; out_len as usize];
                            let _ = memory.read(&store, ptr as usize, &mut out_bytes);
                            
                            if let Ok(json_str) = String::from_utf8(out_bytes) {
                                self.ui_blueprint = serde_json::from_str(&json_str).ok();
                                self.status = "Rendered successfully.".to_string();
                                return;
                            }
                        }
                    }
                }
            }
        }
        self.status = "Wasm Execution Failed.".to_string();
    }
}
