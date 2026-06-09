use serde_json::json;
use std::collections::HashMap;

// =====================================================================
// THE SHARED MEMORY BUFFER
// This is the 1MB bridge between the WebAssembly module and the Rust Host.
// =====================================================================
static mut BUFFER: [u8; 10 * 1024 * 1024] = [0; 10 * 1024 * 1024];

#[unsafe(no_mangle)]
pub extern "C" fn get_buffer_ptr() -> *mut u8 {
    std::ptr::addr_of_mut!(BUFFER).cast::<u8>()
}

// =====================================================================
// HELPER: POLYGON GENERATOR
// =====================================================================
fn build_error_polys<F, G>(x0: f64, x1: f64, exact_f: &F, approx_g: &G) -> (Vec<[f64; 2]>, Vec<[f64; 2]>, Vec<[f64; 2]>) 
where F: Fn(f64) -> f64, G: Fn(f64) -> f64 {
    let mut base_pts = vec![];
    let mut over_pts = vec![];
    let mut under_pts = vec![];
    let steps = 20; 
    
    for j in 0..=steps {
        let px = x0 + (x1 - x0) * (j as f64 / steps as f64);
        let y_ex = exact_f(px);
        let y_ap = approx_g(px);
        let mid = y_ex.min(y_ap); 
        
        base_pts.push([px, mid]);
        over_pts.push([px, y_ap]); 
        under_pts.push([px, y_ex]); 
    }
    
    let mut base_poly = base_pts.clone();
    for j in (0..=steps).rev() { base_poly.push([x0 + (x1 - x0) * (j as f64 / steps as f64), 0.0]); }
    
    let mut over_poly = over_pts.clone();
    for j in (0..=steps).rev() { over_poly.push(base_pts[j]); } 
    
    let mut under_poly = under_pts.clone();
    for j in (0..=steps).rev() { under_poly.push(base_pts[j]); } 
    
    (base_poly, over_poly, under_poly)
}

// =====================================================================
// MAIN EXECUTION ENGINE
// =====================================================================
#[unsafe(no_mangle)]
pub extern "C" fn execute(input_len: usize) -> usize {
    let input_bytes = unsafe { std::slice::from_raw_parts(std::ptr::addr_of!(BUFFER).cast::<u8>(), input_len) };
    
    let payload: serde_json::Value = serde_json::from_slice(input_bytes).unwrap_or_default();
    
    let mut state: HashMap<String, String> = HashMap::new();
    if let Some(inputs) = payload.get("inputs").and_then(|i| i.as_object()) {
        for (k, v) in inputs {
            if let Some(s) = v.as_str() {
                state.insert(k.clone(), s.to_string());
            }
        }
    }
    
    let eq_str = state.get("equation").unwrap_or(&"sin(x) + x".to_string()).clone();
    let a: f64 = state.get("lower_bound").unwrap_or(&"0".to_string()).parse().unwrap_or(0.0);
    let b: f64 = state.get("upper_bound").unwrap_or(&"10".to_string()).parse().unwrap_or(10.0);
    
    // Parse as f64 first to handle UI slider decimals, then cast to i32
    let n: i32 = state.get("intervals")
        .unwrap_or(&"10".to_string())
        .parse::<f64>()
        .unwrap_or(10.0) as i32;

    // Initialize the outputs array FIRST
    let mut outputs = vec![];

    // ==========================================
    // THE BUILT-IN WASM DEBUGGER
    // ==========================================
    let debug_raw_str = std::str::from_utf8(input_bytes).unwrap_or("INVALID_UTF8");
    let msg_1 = format!("DEBUG 1 (Raw Host JSON): {}", debug_raw_str);
    let msg_2 = format!("DEBUG 2 (Parsed State map): {:?}", state);
    let msg_3 = format!("DEBUG 3 (Final N Value): {}", n);

    outputs.push(json!({"type": "text", "value": msg_1, "color": "yellow"}));
    outputs.push(json!({"type": "text", "value": msg_2, "color": "yellow"}));
    outputs.push(json!({"type": "text", "value": msg_3, "color": "red"}));
    // ==========================================

    if let Ok(expr) = eq_str.parse::<meval::Expr>() {
        if let Ok(func) = expr.bind("x") {
            let delta_x = (b - a) / (n as f64);

            let vis_n = n.min(100); 
            let vis_delta_x = (b - a) / (vis_n as f64);

            // 1. Exact Area
            let exact_n = 100_000;
            let dx_ex = (b - a) / (exact_n as f64);
            let mut exact_area = func(a) + func(b);
            for i in 1..exact_n {
                let mult = if i % 2 == 0 { 2.0 } else { 4.0 };
                exact_area += mult * func(a + (i as f64) * dx_ex);
            }
            exact_area *= dx_ex / 3.0;

            let mut exact_curve = vec![];
            for i in 0..=500 {
                let x = a + (b - a) * (i as f64 / 500.0);
                exact_curve.push([x, func(x)]);
            }
            let exact_series = json!({"name": "Exact Curve", "color": "white", "type": "line", "data": exact_curve});

            // 2. Riemann
            let mut r_area = 0.0;
            for i in 0..n {
                let x0 = a + (i as f64) * delta_x;
                r_area += func(x0) * delta_x;
            }
            let r_skew = r_area - exact_area;

            let (mut r_base, mut r_over, mut r_under) = (vec![], vec![], vec![]);
            for i in 0..vis_n {
                let x0 = a + (i as f64) * vis_delta_x;
                let x1 = a + ((i + 1) as f64) * vis_delta_x;
                let y_ap = func(x0);
                let (b_poly, o_poly, u_poly) = build_error_polys(x0, x1, &func, &|_| y_ap);
                r_base.push(b_poly); r_over.push(o_poly); r_under.push(u_poly);
            }

            // 3. Trapezoidal
            let mut t_area = 0.0;
            for i in 0..n {
                let x0 = a + (i as f64) * delta_x;
                let x1 = a + ((i + 1) as f64) * delta_x;
                t_area += (func(x0) + func(x1)) / 2.0 * delta_x;
            }
            let t_skew = t_area - exact_area;

            let (mut t_base, mut t_over, mut t_under) = (vec![], vec![], vec![]);
            for i in 0..vis_n {
                let x0 = a + (i as f64) * vis_delta_x;
                let x1 = a + ((i + 1) as f64) * vis_delta_x;
                let y0 = func(x0); let y1 = func(x1);
                let t_func = |px: f64| y0 + (px - x0) * (y1 - y0) / (x1 - x0);
                let (b_poly, o_poly, u_poly) = build_error_polys(x0, x1, &func, &t_func);
                t_base.push(b_poly); t_over.push(o_poly); t_under.push(u_poly);
            }

            // 4. Simpson's
            let mut n_simp = n;
            if n_simp % 2 != 0 { n_simp += 1; }
            let dx_simp = (b - a) / (n_simp as f64);
            
            let mut s_area = func(a) + func(b);
            for i in 1..n_simp {
                let mult = if i % 2 == 0 { 2.0 } else { 4.0 };
                s_area += mult * func(a + (i as f64) * dx_simp);
            }
            s_area *= dx_simp / 3.0;
            let s_skew = s_area - exact_area;

            let mut vis_simp = vis_n;
            if vis_simp % 2 != 0 { vis_simp += 1; }
            let vis_dx_simp = (b - a) / (vis_simp as f64);
            let (mut s_base, mut s_over, mut s_under) = (vec![], vec![], vec![]);
            
            for i in (0..vis_simp).step_by(2) {
                if i + 1 >= vis_simp { break; }
                let x0 = a + (i as f64) * vis_dx_simp;
                let x1 = a + ((i + 1) as f64) * vis_dx_simp;
                let x2 = a + ((i + 2) as f64) * vis_dx_simp;
                let (y0, y1, y2) = (func(x0), func(x1), func(x2));

                let s_func = |px: f64| {
                    let l0 = ((px - x1) * (px - x2)) / ((x0 - x1) * (x0 - x2));
                    let l1 = ((px - x0) * (px - x2)) / ((x1 - x0) * (x1 - x2));
                    let l2 = ((px - x0) * (px - x1)) / ((x2 - x0) * (x2 - x1));
                    y0 * l0 + y1 * l1 + y2 * l2
                };

                let (b_poly, o_poly, u_poly) = build_error_polys(x0, x2, &func, &s_func);
                s_base.push(b_poly); s_over.push(o_poly); s_under.push(u_poly);
            }
            
            outputs.push(json!({
                "type": "plot", "title": "1. Exact Analytical Curve",
                "series": [exact_series.clone()]
            }));

            outputs.push(json!({
                "type": "plot", "title": format!("2. Left Riemann Sum | Skew: {:+.6}", r_skew),
                "series": [
                    {"name": "Base Area", "color": "blue", "type": "polygon", "data": r_base},
                    {"name": "Overestimate", "color": "yellow", "type": "polygon", "data": r_over},
                    {"name": "Underestimate", "color": "red", "type": "polygon", "data": r_under},
                    {"name": "", "color": "blue", "type": "outlines", "data": r_base},
                    {"name": "", "color": "yellow", "type": "outlines", "data": r_over},
                    {"name": "", "color": "red", "type": "outlines", "data": r_under},
                    exact_series.clone()
                ]
            }));

            outputs.push(json!({
                "type": "plot", "title": format!("3. Trapezoidal Rule | Skew: {:+.6}", t_skew),
                "series": [
                    {"name": "Base Area", "color": "blue", "type": "polygon", "data": t_base},
                    {"name": "Overestimate", "color": "yellow", "type": "polygon", "data": t_over},
                    {"name": "Underestimate", "color": "red", "type": "polygon", "data": t_under},
                    {"name": "", "color": "blue", "type": "outlines", "data": t_base},
                    {"name": "", "color": "yellow", "type": "outlines", "data": t_over},
                    {"name": "", "color": "red", "type": "outlines", "data": t_under},
                    exact_series.clone()
                ]
            }));

            outputs.push(json!({
                "type": "plot", "title": format!("4. Simpson's 1/3 Rule | Skew: {:+.6}", s_skew),
                "series": [
                    {"name": "Base Area", "color": "blue", "type": "polygon", "data": s_base},
                    {"name": "Overestimate", "color": "yellow", "type": "polygon", "data": s_over},
                    {"name": "Underestimate", "color": "red", "type": "polygon", "data": s_under},
                    {"name": "", "color": "blue", "type": "outlines", "data": s_base},
                    {"name": "", "color": "yellow", "type": "outlines", "data": s_over},
                    {"name": "", "color": "red", "type": "outlines", "data": s_under},
                    exact_series.clone()
                ]
            }));

            outputs.push(json!({"type": "text", "value": "--- Execution Log ---", "color": "white"}));
            outputs.push(json!({"type": "text", "value": format!("Exact Area: {:.8}", exact_area), "color": "blue"}));
            outputs.push(json!({"type": "text", "value": format!("Riemann Area: {:.8} (Skew: {:+.8})", r_area, r_skew), "color": "white"}));
            outputs.push(json!({"type": "text", "value": format!("Trapezoid Area: {:.8} (Skew: {:+.8})", t_area, t_skew), "color": "white"}));
            outputs.push(json!({"type": "text", "value": format!("Simpson Area: {:.8} (Skew: {:+.8})", s_area, s_skew), "color": "white"}));

        }
    } else {
        outputs.push(json!({"type": "text", "value": "Error: Invalid Math Syntax", "color": "red"}));
    }

    let blueprint = json!({
        "type": "dashboard",
        "inputs": [
            {"id": "equation", "name": "f(x)=", "type": "text", "default": "sin(x) + x"},
            {"id": "lower_bound", "name": "a =", "type": "text", "default": "0"},
            {"id": "upper_bound", "name": "b =", "type": "text", "default": "10"},
            {"id": "intervals", "name": "N =", "type": "slider", "min": 2.0, "max": 100000.0, "default": "10"}
        ],
        "outputs": outputs
    });
    
    let output_bytes = serde_json::to_string(&blueprint).unwrap().into_bytes();
    unsafe { std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), std::ptr::addr_of_mut!(BUFFER).cast::<u8>(), output_bytes.len()); }
    
    output_bytes.len()
}