use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints}; // Removed unused Polygon warning
use egui_snarl::{InPin, OutPin, Snarl};
use egui_snarl::ui::{PinInfo, SnarlViewer, SnarlStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use wasmtime::*;
use serde::{Serialize, Deserialize};

// ========================================================================================
// THE UNIVERSAL HOST ENGINE: ARCHITECTURE MANIFESTO
// Core Focus: A "Dumb Terminal" Wasm Shell and Universal GPU Renderer
// ========================================================================================
//
// === THE PRIME DIRECTIVE: THE BLIND PAINTER ===
// This engine knows NOTHING about the domain of the modules it runs. It does not know 
// what a MANET, a Logic Gate, or a Cryptographic Hash is. It is strictly an execution 
// environment and a rendering pipeline. 
// IF YOU ARE ADDING DOMAIN-SPECIFIC LOGIC TO THIS FILE, YOU ARE BREAKING THE ARCHITECTURE.
//
// === THE I/O PIPELINE ===
// 1. INPUT: The Host captures user interactions (mouse X/Y, clicks, text input) and 
//    packages them into a generic JSON state dictionary.
// 2. EXECUTION: The Host passes this JSON into the loaded Wasm Cartridge's memory buffer.
// 3. OUTPUT: The Wasm Cartridge computes the physics/logic and returns a JSON "blueprint".
// 4. RENDER: The Host parses the blueprint and draws the requested generic primitives.
//
// === SUPPORTED RENDER ARCHETYPES (THE API) ===
// The Wasm blueprint dictates the layout. The Host currently supports:
// - "dashboard": A generic UI layout for sliders, text inputs, and 2D Plotting (egui_plot).
// - "tabbed_studio": A multi-tab interface for complex modules.
//      -> Tab Type "text": Renders raw strings (e.g., Verilog code, TCL scripts, Tables).
//      -> Tab Type "node_graph": Renders an interactive node-based UI (egui_snarl).
//      -> Tab Type "custom_canvas": A pure 2D GPU painter. Reads a `draw_list` of generic 
//         shapes (circle, line, text, arrow) and renders them exactly at the given X/Y.
//
// === FUTURE EXPANSION: THE 3D SCENE ===
// When 3D is required, we will add a "3d_scene" tab type here. The Host Engine will 
// handle the native camera panning, zooming, and 3D projection rendering. Wasm cartridges 
// will simply pass arrays of (X, Y, Z) coordinates, radii, and colors.
//
// === THE GOLDEN RULES ===
// Write it once, run any module.
// This is based on "Global" if else design, never delete something before asking
// ========================================================================================

// --- UNIVERSAL NODE GRAPH STRUCTS ---
#[derive(Clone, Serialize, Deserialize)]
struct DumbNode { 
    name: String, 
    inputs: Vec<String>, 
    outputs: Vec<String>,
    
    // --- NEW: GENERIC UI PAYLOADS ---
    #[serde(default)]
    show_toggle: bool,
    #[serde(default)]
    toggle_state: bool,
    
    #[serde(default)]
    show_text_input: bool,
    #[serde(default)]
    custom_text: String,
}

struct DumbViewer {
    pub trigger_update: bool, // Tells the Host to re-fire Wasm when a user clicks something
}

impl SnarlViewer<DumbNode> for DumbViewer {
    fn title(&mut self, node: &DumbNode) -> String { 
        if node.show_text_input && !node.custom_text.is_empty() {
            format!("{} ({})", node.name, node.custom_text)
        } else {
            node.name.clone()
        }
    }
    
    fn inputs(&mut self, node: &DumbNode) -> usize { node.inputs.len() }
    fn outputs(&mut self, node: &DumbNode) -> usize { node.outputs.len() }
    
    // RENDER GENERIC NODE UI (Checkboxes & Text Inputs)
    fn show_header(&mut self, node: egui_snarl::NodeId, inputs: &[InPin], outputs: &[OutPin], ui: &mut egui::Ui, scale: f32, snarl: &mut Snarl<DumbNode>) {
        let n = &mut snarl[node];
        ui.horizontal(|ui| {
            if n.show_toggle {
                if ui.checkbox(&mut n.toggle_state, "").changed() {
                    self.trigger_update = true; // Instantly recalculate!
                }
            }
            if n.show_text_input {
                let prev_text = n.custom_text.clone();
                ui.add(egui::TextEdit::singleline(&mut n.custom_text).desired_width(50.0));
                if prev_text != n.custom_text {
                    self.trigger_update = true; // Instantly recalculate!
                }
            }
            ui.label(self.title(n));
        });
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut egui::Ui, _scale: f32, snarl: &mut Snarl<DumbNode>) -> PinInfo {
        ui.label(&snarl[pin.id.node].inputs[pin.id.input]); 
        PinInfo::circle().with_fill(egui::Color32::GRAY)
    }
    
    fn show_output(&mut self, pin: &OutPin, ui: &mut egui::Ui, _scale: f32, snarl: &mut Snarl<DumbNode>) -> PinInfo {
        ui.label(&snarl[pin.id.node].outputs[pin.id.output]); 
        
        // GENERIC VISUAL FEEDBACK: If the module says this node is "active" (toggle_state = true), make it green!
        let color = if snarl[pin.id.node].toggle_state { egui::Color32::GREEN } else { egui::Color32::RED };
        PinInfo::circle().with_fill(color)
    }
    
    fn has_node_menu(&mut self, _node: &DumbNode) -> bool { true }
    fn show_node_menu(&mut self, node: egui_snarl::NodeId, _inputs: &[InPin], _outputs: &[OutPin], ui: &mut egui::Ui, _scale: f32, snarl: &mut Snarl<DumbNode>) {
        if ui.button("🗑 Delete Node").clicked() { snarl.remove_node(node); self.trigger_update = true; ui.close_menu(); }
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
                                self.user_inputs.clear(); self.snarl = Snarl::new(); self.active_tab_id.clear(); 
                                
                                // --- NEW: STATE HYDRATION (Load custom ICs) ---
                                let base_name = mod_name.trim_end_matches(".wasm");
                                let data_dir = Path::new("modules").join(format!("{}_data", base_name));
                                if let Ok(entries) = fs::read_dir(data_dir) {
                                    for entry in entries.flatten() {
                                        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                                let filename = entry.file_name().to_string_lossy().to_string();
                                                self.user_inputs.insert(format!("ic_{}", filename), content);
                                            }
                                        }
                                    }
                                }
                                // ----------------------------------------------
                                trigger_wasm = true; 
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
                // SHAPE 2: THE TABBED STUDIO
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
                            egui::CentralPanel::default().show_inside(ui, |ui| { 
    let mut viewer = DumbViewer { trigger_update: false };
    self.snarl.show(&mut viewer, &SnarlStyle::new(), "snarl_canvas", ui); 
    if viewer.trigger_update { trigger_wasm = true; } // Live execution!
});
                        } 
                        else if tab_kind == "text" {
                            let text = active_tab_data["value"].as_str().unwrap_or("");
                            let mut display_text = text.to_string();
                            egui::ScrollArea::vertical().show(ui, |ui| { ui.add(egui::TextEdit::multiline(&mut display_text).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(30)); });
                        }
                        else if tab_kind == "custom_canvas" {
                            egui::ScrollArea::both().show(ui, |ui| {
                                let (rect, _response) = ui.allocate_exact_size(egui::vec2(800.0, 600.0), egui::Sense::click_and_drag());
                                let painter = ui.painter_at(rect);
                                
                                if let Some(bg) = active_tab_data.get("background").and_then(|b| b.as_array()) {
                                    let color = egui::Color32::from_rgb(bg[0].as_u64().unwrap_or(0) as u8, bg[1].as_u64().unwrap_or(0) as u8, bg[2].as_u64().unwrap_or(0) as u8);
                                    painter.rect_filled(rect, 0.0, color);
                                }

                                if let Some(draw_list) = active_tab_data.get("draw_list").and_then(|d| d.as_array()) {
                                    for cmd in draw_list {
                                        let shape = cmd["shape"].as_str().unwrap_or("");
                                        
                                        let get_color = |key: &str, default: egui::Color32| -> egui::Color32 {
                                            if let Some(arr) = cmd.get(key).and_then(|a| a.as_array()) {
                                                if arr.len() >= 3 {
                                                    let a = if arr.len() == 4 { arr[3].as_u64().unwrap_or(255) as u8 } else { 255 };
                                                    return egui::Color32::from_rgba_unmultiplied(
                                                        arr[0].as_u64().unwrap_or(0) as u8, arr[1].as_u64().unwrap_or(0) as u8, arr[2].as_u64().unwrap_or(0) as u8, a
                                                    );
                                                }
                                            }
                                            default
                                        };

                                        if shape == "circle" {
                                            let x = cmd["x"].as_f64().unwrap_or(0.0) as f32;
                                            let y = cmd["y"].as_f64().unwrap_or(0.0) as f32;
                                            let r = cmd["radius"].as_f64().unwrap_or(10.0) as f32;
                                            let pos = rect.min + egui::vec2(x, y);
                                            
                                            let fill = get_color("fill", egui::Color32::TRANSPARENT);
                                            let stroke_color = get_color("stroke", egui::Color32::TRANSPARENT);
                                            let stroke_width = cmd["stroke_width"].as_f64().unwrap_or(0.0) as f32;
                                            
                                            painter.circle(pos, r, fill, egui::Stroke::new(stroke_width, stroke_color));
                                        } 
                                        else if shape == "text" {
                                            let x = cmd["x"].as_f64().unwrap_or(0.0) as f32;
                                            let y = cmd["y"].as_f64().unwrap_or(0.0) as f32;
                                            let text = cmd["text"].as_str().unwrap_or("");
                                            let size = cmd["size"].as_f64().unwrap_or(14.0) as f32;
                                            let pos = rect.min + egui::vec2(x, y);
                                            let color = get_color("color", egui::Color32::WHITE);
                                            
                                            painter.text(pos, egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(size), color);
                                        }
                                        else if shape == "arrow" || shape == "line" {
                                            if let (Some(from), Some(to)) = (cmd.get("from").and_then(|a| a.as_array()), cmd.get("to").and_then(|a| a.as_array())) {
                                                let p1 = rect.min + egui::vec2(from[0].as_f64().unwrap_or(0.0) as f32, from[1].as_f64().unwrap_or(0.0) as f32);
                                                let p2 = rect.min + egui::vec2(to[0].as_f64().unwrap_or(0.0) as f32, to[1].as_f64().unwrap_or(0.0) as f32);
                                                let color = get_color("color", egui::Color32::WHITE);
                                                let width = cmd["width"].as_f64().unwrap_or(1.0) as f32;
                                                
                                                if shape == "arrow" {
                                                    painter.arrow(p1, p2 - p1, egui::Stroke::new(width, color));
                                                } else {
                                                    painter.line_segment([p1, p2], egui::Stroke::new(width, color));
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        else if tab_kind == "dashboard" {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.add_space(10.0);
                                let controls = active_tab_data.get("inputs").or_else(|| active_tab_data.get("controls")).and_then(|c| c.as_array());
                                
                                if let Some(controls) = controls {
                                    for ctrl in controls {
                                        let id = ctrl["id"].as_str().unwrap();
                                        let label = ctrl.get("label").or_else(|| ctrl.get("name")).and_then(|v| v.as_str()).unwrap_or(id);
                                        let kind = ctrl["type"].as_str().unwrap();
                                        
                                        let val = self.user_inputs.entry(id.to_string()).or_insert_with(|| ctrl.get("default").and_then(|d| d.as_str()).unwrap_or("").to_string());

                                        ui.horizontal(|ui| {
                                            ui.label(label);
                                            if kind == "text" { ui.text_edit_singleline(val); } 
                                        });
                                    }
                                }
                                ui.add_space(15.0);
                                if ui.button("⚡ Save to Disk").clicked() { trigger_wasm = true; }
                            });
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
                                if let Ok(blueprint) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    
                                    // --- NEW: COMMAND-DRIVEN I/O (LAZY EVALUATION) ---
                                    if let Some(commands) = blueprint.get("host_commands").and_then(|c| c.as_array()) {
                                        let base_name = mod_name.trim_end_matches(".wasm");
                                        let data_dir = Path::new("modules").join(format!("{}_data", base_name));
                                        let mut retrigger = false;

                                        for cmd in commands {
                                            let action = cmd.get("action").and_then(|a| a.as_str()).unwrap_or("");
                                            let filename = cmd.get("filename").and_then(|f| f.as_str()).unwrap_or("data.json");

                                            if action == "write" {
                                                let _ = fs::create_dir_all(&data_dir);
                                                if let Some(data) = cmd.get("data").and_then(|d| d.as_str()) {
                                                    let _ = fs::write(data_dir.join(filename), data);
                                                }
                                            } else if action == "read" {
                                                let input_key = format!("file_{}", filename);
                                                if !self.user_inputs.contains_key(&input_key) {
                                                    if let Ok(content) = fs::read_to_string(data_dir.join(filename)) {
                                                        self.user_inputs.insert(input_key, content);
                                                        retrigger = true; 
                                                    }
                                                }
                                            }
                                        }
                                        
                                        if retrigger {
                                            self.fire_wasm();
                                            return;
                                        }
                                    }
                                    // ------------------------------------------------

                                    self.ui_blueprint = Some(blueprint);
                                    self.status = "Rendered successfully.".to_string();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        self.status = "Wasm Execution Failed.".to_string();
    }
}
