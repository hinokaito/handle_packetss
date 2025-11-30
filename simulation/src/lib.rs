//! # Simulation Module
//!
//! This is the core WebAssembly (Wasm) module for the packet traffic simulation.
//! It serves as the bridge between JavaScript (browser) and Rust (Wasm),
//! enabling high-performance computation in the browser environment.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────┐      ┌──────────────────┐      ┌─────────────────┐
//! │   Browser JS    │ ──── │   This Module    │ ──── │   Go Server     │
//! │   (Frontend)    │      │   (Rust/Wasm)    │      │   (WebSocket)   │
//! └─────────────────┘      └──────────────────┘      └─────────────────┘
//! ```
//!
//! ## Current Status
//!
//! This module is in **Step 1** of development:
//! - [x] Basic JS <-> Wasm interop
//! - [x] Console logging from Rust
//! - [x] Message handling from WebSocket
//! - [ ] WebGPU/WebGL rendering (TODO)
//! - [ ] Binary protocol parsing (TODO)
//! - [ ] Simulation logic (TODO)
//!
//! ## Future Plans
//!
//! - Implement particle system for 100k+ packet visualization
//! - Add wgpu-based WebGPU/WebGL rendering
//! - Create load balancing algorithm simulations

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Represents a single packet in the simulation.
///
/// This struct mirrors the Go server's Packet struct.
/// It's used to deserialize JSON data received via WebSocket.
///
/// # JSON Format
///
/// ```json
/// {"id": 1, "x": 10.5, "y": 20.0}
/// ```
///
/// # Fields
///
/// * `id` - Unique identifier for the packet
/// * `x` - X coordinate (0.0 - 800.0, matching canvas width)
/// * `y` - Y coordinate (0.0 - 600.0, matching canvas height)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}

// =============================================================================
// JAVASCRIPT BINDINGS (FFI - Foreign Function Interface)
// =============================================================================

/// This `extern "C"` block declares functions that exist in JavaScript.
/// `wasm-bindgen` generates the glue code to call these JS functions from Rust.
///
/// # How it works
///
/// 1. We declare the function signature here in Rust
/// 2. `wasm-bindgen` generates JavaScript wrapper code
/// 3. When Rust calls `log()`, it actually invokes `console.log()` in the browser
///
/// # Example
///
/// ```rust
/// log("Hello from Rust!"); // This prints to browser's console
/// ```
#[wasm_bindgen]
extern "C" {
    /// Binding to JavaScript's `console.log()` function.
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    /// Binding to JavaScript's `drawPacket()` function.
    /// This function is defined in index.html and draws a white square on the canvas.
    ///
    /// # Parameters
    ///
    /// * `x` - X coordinate on the canvas
    /// * `y` - Y coordinate on the canvas
    #[wasm_bindgen(js_namespace = window)]
    fn drawPacket(x: f64, y: f64);

    /// Binding to JavaScript's `drawPackets()` function.
    /// Draws multiple packets at once for better performance.
    ///
    /// # Parameters
    ///
    /// * `coords` - Float64Array containing [x0, y0, x1, y1, x2, y2, ...]
    #[wasm_bindgen(js_namespace = window)]
    fn drawPackets(coords: &[f64]);

    /// Binding to JavaScript's `performance.now()` for timing.
    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

// =============================================================================
// PUBLIC API - Functions exported to JavaScript
// =============================================================================

/// Logs a message to the browser console from Rust/Wasm.
///
/// This function is exported to JavaScript and can be called from the browser.
/// It's a simple wrapper around the `console.log()` binding.
///
/// # Parameters
///
/// * `message` - The message string to display in the browser console
///
/// # Example (JavaScript)
///
/// ```javascript
/// import { console_log } from './pkg/simulation.js';
/// console_log("Hello from JavaScript, through Rust!");
/// ```
///
/// # Why use this instead of console.log directly?
///
/// This function demonstrates Wasm interop. In the future, this could be
/// extended to:
/// - Format messages with additional context
/// - Filter log levels
/// - Send logs to a remote server
#[wasm_bindgen]
pub fn console_log(message: &str) {
    log(message);
}

/// Processes a message received from the WebSocket connection.
///
/// This is the main entry point for handling real-time data from the Go server.
/// Currently, it just logs the received message, but this will be extended to:
///
/// - Parse binary packet data
/// - Update simulation state
/// - Trigger rendering updates
///
/// # Parameters
///
/// * `message` - The message string received from the WebSocket
///
/// # Data Flow
///
/// ```text
/// Go Server ──WebSocket──> Browser JS ──> handle_message() ──> Process/Render
/// ```
///
/// # Example (JavaScript)
///
/// ```javascript
/// ws.onmessage = (event) => {
///     handle_message(event.data);  // Pass WebSocket data to Rust
/// };
/// ```
///
/// # Future Implementation
///
/// ```rust
/// // TODO: Parse binary packet format
/// // let packets: Vec<Packet> = parse_binary(message);
/// // simulation_state.update(packets);
/// // renderer.queue_draw();
/// ```
#[wasm_bindgen]
pub fn handle_message(message: &str) {
    // Log message size for performance analysis
    let msg_size = message.len();
    log(&format!(
        "[Rust/Wasm] Received: {} bytes ({:.2} KB)",
        msg_size,
        msg_size as f64 / 1024.0
    ));

    // Try to parse as JSON array first (multiple packets)
    let start_parse = now();
    if let Ok(packets) = serde_json::from_str::<Vec<Packet>>(message) {
        let parse_time = now() - start_parse;

        log(&format!(
            "[Rust/Wasm] Parsed {} packets in {:.2}ms",
            packets.len(),
            parse_time
        ));

        // Convert packets to flat coordinate array for batch drawing
        let start_convert = now();
        let coords: Vec<f64> = packets
            .iter()
            .flat_map(|p| [p.x, p.y])
            .collect();
        let convert_time = now() - start_convert;

        // Draw all packets at once
        let start_draw = now();
        drawPackets(&coords);
        let draw_time = now() - start_draw;

        // Performance summary
        log(&format!(
            "[Rust/Wasm] Performance: parse={:.2}ms, convert={:.2}ms, draw={:.2}ms, total={:.2}ms",
            parse_time,
            convert_time,
            draw_time,
            parse_time + convert_time + draw_time
        ));
        log(&format!(
            "[Rust/Wasm] JSON overhead: {:.2} bytes/packet",
            msg_size as f64 / packets.len() as f64
        ));

        return;
    }

    // Try to parse as single Packet
    match serde_json::from_str::<Packet>(message) {
        Ok(packet) => {
            log(&format!(
                "[Rust/Wasm] Parsed single Packet: id={}, x={}, y={}",
                packet.id, packet.x, packet.y
            ));
            drawPacket(packet.x, packet.y);
        }
        Err(_) => {
            // Plain text message (like "Hello")
            log(&format!("[Rust/Wasm] Plain text: {}", message));
        }
    }
}

/// Entry point for the Wasm module.
///
/// This function is automatically called when the Wasm module is loaded.
/// The `#[wasm_bindgen(start)]` attribute marks this as the module's
/// initialization function.
///
/// # Lifecycle
///
/// 1. Browser loads `simulation.js`
/// 2. `init()` is called (from JavaScript)
/// 3. Wasm binary is fetched and compiled
/// 4. This `main()` function runs automatically
/// 5. Module is ready for use
///
/// # Current Implementation
///
/// Just logs an initialization message. In the future, this will:
///
/// - Initialize WebGPU/WebGL context
/// - Set up rendering pipelines
/// - Allocate memory for particle buffers
/// - Start the render loop
#[wasm_bindgen(start)]
pub fn main() {
    // Log initialization message to confirm the module loaded successfully
    log("[Rust/Wasm] Module initialized!");

    // TODO: Future initialization steps:
    // - init_webgpu_context()
    // - create_render_pipeline()
    // - allocate_particle_buffers()
    // - start_animation_loop()
}

/// バイナリデータをパースする関数
#[wasm_bindgen]
pub fn handle_binary(data: &[u8]) {
    // 8バイト = 1パケット
    let packet_count = data.len() / 8;
    
    let mut coords: Vec<f64> = Vec::with_capacity(packet_count * 2);
    
    for i in 0..packet_count {
        let offset = i * 8;
        
        // ID (4 bytes) - 今回は使わない
        // let id = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        
        // X (2 bytes) → f64 に復元
        let x16 = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
        let x = (x16 as f64) * 800.0 / 65535.0;
        
        // Y (2 bytes) → f64 に復元
        let y16 = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
        let y = (y16 as f64) * 600.0 / 65535.0;
        
        coords.push(x);
        coords.push(y);
    }
    
    drawPackets(&coords);
}