use serde_json::json;

static mut BUFFER: [u8; 1024 * 1024] = [0; 1024 * 1024];

#[unsafe(no_mangle)]
pub extern "C" fn get_buffer_ptr() -> *mut u8 {
    std::ptr::addr_of_mut!(BUFFER).cast::<u8>()
}

fn trace_math(target_node: &str, target_pin: u64, nodes: &serde_json::Map<String, serde_json::Value>, wires: &[serde_json::Value]) -> String {
    for wire in wires {
        let in_pin = &wire["in_pin"];
        if in_pin["node"].as_u64().unwrap().to_string() == target_node && in_pin["input"].as_u64().unwrap() == target_pin {
            let src_id = wire["out_pin"]["node"].as_u64().unwrap().to_string();
            let src_name = nodes[&src_id]["value"]["name"].as_str().unwrap();
            let out_pin_idx = wire["out_pin"]["output"].as_u64().unwrap();
            
            return match src_name {
                "Input A" => "A".to_string(), "Input B" => "B".to_string(), "Clock (clk)" => "clk".to_string(),
                "NOT Gate" => format!("~{}", trace_math(&src_id, 0, nodes, wires)),
                "AND Gate" => format!("({} & {})", trace_math(&src_id, 0, nodes, wires), trace_math(&src_id, 1, nodes, wires)),
                "NAND Gate" => format!("~({} & {})", trace_math(&src_id, 0, nodes, wires), trace_math(&src_id, 1, nodes, wires)),
                "OR Gate" => format!("({} | {})", trace_math(&src_id, 0, nodes, wires), trace_math(&src_id, 1, nodes, wires)),
                "NOR Gate" => format!("~({} | {})", trace_math(&src_id, 0, nodes, wires), trace_math(&src_id, 1, nodes, wires)),
                "XOR Gate" => format!("({} ^ {})", trace_math(&src_id, 0, nodes, wires), trace_math(&src_id, 1, nodes, wires)),
                "XNOR Gate" => format!("~({} ^ {})", trace_math(&src_id, 0, nodes, wires), trace_math(&src_id, 1, nodes, wires)),
                "D-Flip Flop" => if out_pin_idx == 0 { "Q".to_string() } else { "(~Q)".to_string() },
                _ => "[?]".to_string(),
            };
        }
    }
    "0".to_string()
}

fn simulate_graph(target_node: &str, target_pin: u64, nodes: &serde_json::Map<String, serde_json::Value>, wires: &[serde_json::Value], a_val: bool, b_val: bool, q_val: bool) -> bool {
    for wire in wires {
        let in_pin = &wire["in_pin"];
        if in_pin["node"].as_u64().unwrap().to_string() == target_node && in_pin["input"].as_u64().unwrap() == target_pin {
            let src_id = wire["out_pin"]["node"].as_u64().unwrap().to_string();
            let src_name = nodes[&src_id]["value"]["name"].as_str().unwrap();
            let out_pin_idx = wire["out_pin"]["output"].as_u64().unwrap();

            return match src_name {
                "Input A" => a_val, "Input B" => b_val, "Clock (clk)" => false,
                "NOT Gate" => !simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val),
                "AND Gate" => simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val) & simulate_graph(&src_id, 1, nodes, wires, a_val, b_val, q_val),
                "OR Gate"  => simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val) | simulate_graph(&src_id, 1, nodes, wires, a_val, b_val, q_val),
                "XOR Gate" => simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val) ^ simulate_graph(&src_id, 1, nodes, wires, a_val, b_val, q_val),
                "NAND Gate"=> !(simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val) & simulate_graph(&src_id, 1, nodes, wires, a_val, b_val, q_val)),
                "NOR Gate" => !(simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val) | simulate_graph(&src_id, 1, nodes, wires, a_val, b_val, q_val)),
                "XNOR Gate"=> !(simulate_graph(&src_id, 0, nodes, wires, a_val, b_val, q_val) ^ simulate_graph(&src_id, 1, nodes, wires, a_val, b_val, q_val)),
                "D-Flip Flop" => if out_pin_idx == 0 { q_val } else { !q_val },
                _ => false,
            };
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn execute(input_len: usize) -> usize {
    let input_bytes = unsafe { std::slice::from_raw_parts(std::ptr::addr_of!(BUFFER).cast::<u8>(), input_len) };
    let state: serde_json::Value = serde_json::from_slice(input_bytes).unwrap_or_default();
    
    let mut verilog_code = String::from("module GeneratedLogic(input wire A, input wire B, input wire clk, output wire Out);\n");
    let mut internal_wires = Vec::new();
    let mut assign_statements = Vec::new();
    let mut out_node_id = String::new();
    let mut ff_node_id = String::new();

    if let Some(graph) = state.get("graph") {
        let nodes = graph.get("nodes").and_then(|n| n.as_object());
        let wires = graph.get("wires").and_then(|w| w.as_array());

        if let (Some(nodes), Some(wires)) = (nodes, wires) {
            
            let get_source = |target_node_id: &str, target_pin_idx: u64| -> String {
                for wire in wires {
                    let in_pin = &wire["in_pin"];
                    if in_pin["node"].as_u64().unwrap().to_string() == target_node_id && in_pin["input"].as_u64().unwrap() == target_pin_idx {
                        let source_id = wire["out_pin"]["node"].as_u64().unwrap().to_string();
                        let source_name = nodes[&source_id]["value"]["name"].as_str().unwrap();
                        let out_pin_idx = wire["out_pin"]["output"].as_u64().unwrap();
                        return match source_name {
                            "Input A" => "A".to_string(), "Input B" => "B".to_string(), "Clock (clk)" => "clk".to_string(),
                            "D-Flip Flop" => if out_pin_idx == 0 { "Q".to_string() } else { "Q_bar".to_string() },
                            _ => format!("wire_{}", source_id),
                        };
                    }
                }
                "1'b0".to_string()
            };

            for (node_id, node_data) in nodes {
                let name = node_data["value"]["name"].as_str().unwrap();
                let logic_op = match name {
                    "AND Gate" => Some("&"), "OR Gate" => Some("|"), "XOR Gate" => Some("^"),
                    "NAND Gate" => Some("~&"), "NOR Gate" => Some("~|"), "XNOR Gate" => Some("~^"), "NOT Gate" => Some("~"), _ => None,
                };

                if let Some(op) = logic_op {
                    internal_wires.push(format!("wire_{}", node_id));
                    let input_0 = get_source(node_id, 0);
                    if name == "NOT Gate" {
                        assign_statements.push(format!("  assign wire_{} = {} {};", node_id, op, input_0));
                    } else {
                        let input_1 = get_source(node_id, 1);
                        assign_statements.push(format!("  assign wire_{} = {} {} {};", node_id, input_0, op, input_1));
                    }
                }
                if name == "D-Flip Flop" { ff_node_id = node_id.to_string(); }
                if name == "Module OUT" { 
                    out_node_id = node_id.to_string();
                    assign_statements.push(format!("  assign Out = {};", get_source(node_id, 0))); 
                }
            }

            let mut table_content = String::new();
            let mut states_array = vec![];
            let mut transitions_array = vec![];
            
            if !ff_node_id.is_empty() {
                table_content.push_str("=== STATE TABLE ===\n");
                table_content.push_str(" Current Q | Input A | Input B || Next Q* | Output \n");
                table_content.push_str("-----------|---------|---------||---------|--------\n");
                
                states_array.push("Q=0");
                states_array.push("Q=1");

                for q in [false, true] {
                    for a in [false, true] {
                        for b in [false, true] {
                            let next_q = simulate_graph(&ff_node_id, 0, nodes, wires, a, b, q);
                            let out_val = simulate_graph(&out_node_id, 0, nodes, wires, a, b, q);
                            table_content.push_str(&format!("     {}     |    {}    |    {}    ||    {}    |   {}   \n", q as u8, a as u8, b as u8, next_q as u8, out_val as u8));
                        }
                    }
                    table_content.push_str("-----------|---------|---------||---------|--------\n");

                    let next_q_0 = simulate_graph(&ff_node_id, 0, nodes, wires, false, false, q);
                    let next_q_1 = simulate_graph(&ff_node_id, 0, nodes, wires, true, false, q);
                    
                    let from_str = if q { "Q=1" } else { "Q=0" };
                    let to_str_0 = if next_q_0 { "Q=1" } else { "Q=0" };
                    let to_str_1 = if next_q_1 { "Q=1" } else { "Q=0" };

                    if to_str_0 == to_str_1 {
                        transitions_array.push(json!({"from": from_str, "to": to_str_0, "label": "clk"}));
                    } else {
                        transitions_array.push(json!({"from": from_str, "to": to_str_0, "label": "clk (A=0)"}));
                        transitions_array.push(json!({"from": from_str, "to": to_str_1, "label": "clk (A=1)"}));
                    }
                }
            } else {
                table_content.push_str("=== TRUTH TABLE ===\n");
                table_content.push_str(" Input A | Input B ||  Output \n");
                table_content.push_str("---------|---------||---------\n");
                for a in [false, true] {
                    for b in [false, true] {
                        let out_val = simulate_graph(&out_node_id, 0, nodes, wires, a, b, false);
                        table_content.push_str(&format!("    {}    |    {}    ||    {}    \n", a as u8, b as u8, out_val as u8));
                    }
                }
            }

            if !internal_wires.is_empty() { verilog_code.push_str(&format!("  wire {};\n\n", internal_wires.join(", "))); }
            if !ff_node_id.is_empty() {
                verilog_code.push_str("  reg Q = 1'b0;\n  wire Q_bar;\n  assign Q_bar = ~Q;\n\n");
            }
            for stmt in assign_statements { verilog_code.push_str(&format!("{}\n", stmt)); }
            if !ff_node_id.is_empty() {
                verilog_code.push_str("\n  always @(posedge clk) begin\n");
                let d_input_str = trace_math(&ff_node_id, 0, nodes, wires);
                verilog_code.push_str(&format!("    Q <= {};\n", d_input_str));
                verilog_code.push_str("  end\n");
            }
            verilog_code.push_str("endmodule\n");

            let blueprint = json!({
                "type": "tabbed_studio", 
                "tabs": [
                    {
                        "id": "canvas", "title": "🎨 Node Canvas", "type": "node_graph",
                        "palette": [
                            {"name": "Input A", "inputs": [], "outputs": ["Out"]},
                            {"name": "Input B", "inputs": [], "outputs": ["Out"]},
                            {"name": "Clock (clk)", "inputs": [], "outputs": ["Out"]},
                            {"name": "AND Gate", "inputs": ["A", "B"], "outputs": ["Out"]},
                            {"name": "NAND Gate", "inputs": ["A", "B"], "outputs": ["Out"]},
                            {"name": "OR Gate", "inputs": ["A", "B"], "outputs": ["Out"]},
                            {"name": "NOR Gate", "inputs": ["A", "B"], "outputs": ["Out"]},
                            {"name": "XOR Gate", "inputs": ["A", "B"], "outputs": ["Out"]},
                            {"name": "XNOR Gate", "inputs": ["A", "B"], "outputs": ["Out"]},
                            {"name": "NOT Gate", "inputs": ["In"], "outputs": ["Out"]},
                            {"name": "D-Flip Flop", "inputs": ["D", "clk"], "outputs": ["Q", "Q_bar"]},
                            {"name": "Module OUT", "inputs": ["Val"], "outputs": []}
                        ]
                    },
                    { "id": "verilog", "title": "📝 Verilog RTL", "type": "text", "value": verilog_code },
                    { "id": "tables", "title": "📊 Data Tables", "type": "text", "value": table_content },
                    { "id": "diagram", "title": "🔄 State Diagram", "type": "state_diagram", "states": states_array, "transitions": transitions_array },
                    { "id": "math", "title": "🗺️ Equations", "type": "text", "value": format!("Boolean Algebra:\n\nOut = {}\n\n(If sequential, D = {})", trace_math(&out_node_id, 0, nodes, wires), if !ff_node_id.is_empty() { trace_math(&ff_node_id, 0, nodes, wires) } else { "N/A".to_string() }) }
                ]
            });

            let output_json = serde_json::to_string(&blueprint).unwrap();
            let output_bytes = output_json.as_bytes();
            unsafe { std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), std::ptr::addr_of_mut!(BUFFER).cast::<u8>(), output_bytes.len()); }
            return output_bytes.len();
        }
    }
    0
}
