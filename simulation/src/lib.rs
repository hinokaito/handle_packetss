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

use wasm_bindgen::prelude::*;

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
    ///
    /// The `js_namespace = console` attribute tells wasm-bindgen that this
    /// function belongs to the `console` object in JavaScript.
    ///
    /// # Parameters
    ///
    /// * `s` - The string message to log to the browser console
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
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
    // Log the received message with a prefix to identify it came from Wasm
    log(&format!("[Rust/Wasm] Received: {}", message));

    // TODO: In the future, this will:
    // 1. Deserialize the message (JSON or binary)
    // 2. Update the internal simulation state
    // 3. Trigger a re-render of the visualization
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

// =============================================================================
// INTERNAL HELPERS (Not exported to JavaScript)
// =============================================================================

// TODO: Add internal helper functions here as the project grows
//
// Examples of future functions:
//
// /// Parses a binary packet from the server
// fn parse_packet(data: &[u8]) -> Packet { ... }
//
// /// Updates the simulation state with new packet data
// fn update_simulation(packets: Vec<Packet>) { ... }
//
// /// Renders the current state using WebGPU
// fn render_frame() { ... }
