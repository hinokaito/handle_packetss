// =============================================================================
// LIB.RS - エントリーポイント・API担当
// JSとのつなぎ込み（wasm_bindgen）、グローバル変数管理
// =============================================================================

mod renderer;
mod simulation;

use renderer::{init_gpu_internal, render_frame_internal, render_packets_gpu, render_simulation_frame_internal};
use simulation::{SimulationState, WIDTH, HEIGHT};

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

// =============================================================================
// GLOBAL STATE - グローバル変数管理
// =============================================================================

// JavaScriptからRustへ大量のデータを渡す際や、計算結果を一時的に保持するための「使いまわし可能なメモリ領域」
thread_local! {
    // JSとRust間でデータをやり取りするための一時的な共有メモリバッファ
    static PACKET_BUFFER: RefCell<Vec<f32>> = RefCell::new(Vec::new());
}

// シミュレーション状態をグローバルに保持（JSから複数回アクセスするため）
thread_local! {
    static SIMULATION_STATE: RefCell<Option<SimulationState>> = RefCell::new(None);
}

// =============================================================================
// JS INTERFACE - 外部関数宣言
// =============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

// =============================================================================
// WASM ENTRY POINT
// =============================================================================

// Wasmモジュール読み込み時に自動実行されるエントリーポイント
#[wasm_bindgen(start)]
pub fn main() {
    log("[Rust/Wasm] Module initialized!");
}

// =============================================================================
// GPU INITIALIZATION API
// =============================================================================

// JSから呼び出されるWebGPU初期化のエントリーポイント。非同期処理のPromiseを返す
#[wasm_bindgen]
pub fn init_gpu(canvas_id: &str) -> JsValue {
    let canvas_id = canvas_id.to_string();
    wasm_bindgen_futures::future_to_promise(async move {
        init_gpu_internal(&canvas_id)
            .await
            .map(|_| JsValue::TRUE)
            .map_err(|e| e)
    })
    .into()
}

// =============================================================================
// RENDERING API
// =============================================================================

// アニメーションフレームごとに呼び出され、画面を再描画する関数
#[wasm_bindgen]
pub fn render_frame() {
    render_frame_internal();
}

// =============================================================================
// PACKET BUFFER API - JSとの共有メモリ管理
// =============================================================================

// 共有バッファのメモリアドレス（ポインタ）をJSに返す関数
#[wasm_bindgen]
pub fn get_packet_buffer_ptr() -> *const f32 {
    PACKET_BUFFER.with(|buffer| buffer.borrow().as_ptr())
}

// 共有バッファの現在の長さをJSに返す関数
#[wasm_bindgen]
pub fn get_packet_buffer_len() -> usize {
    PACKET_BUFFER.with(|buffer| buffer.borrow().len())
}

// 共有バッファのメモリ領域を指定サイズ分確保する関数
#[wasm_bindgen]
pub fn allocate_packet_buffer(capacity: usize) {
    PACKET_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();
        buf.reserve(capacity * 2);
        log(&format!(
            "[Rust/Wasm] Allocated packet buffer with capacity for {} packets ({} bytes)",
            capacity,
            capacity * 2 * std::mem::size_of::<f32>()
        ));
    });
}

// 共有バッファの内容をクリアする関数
#[wasm_bindgen]
pub fn clear_packet_buffer() {
    PACKET_BUFFER.with(|buffer| {
        buffer.borrow_mut().clear();
    });
}

// バイナリデータからパケット情報を読み取り、共有バッファを更新する関数
#[wasm_bindgen]
pub fn update_packet_buffer_from_binary(data: &[u8]) -> usize {
    let packet_count = data.len() / 8;

    PACKET_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();

        let required = packet_count * 2;
        let current_capacity = buf.capacity();
        if current_capacity < required {
            buf.reserve(required - current_capacity);
        }

        for i in 0..packet_count {
            let offset = i * 8;

            let x16 = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
            let x = (x16 as f32) * WIDTH / 65535.0;

            let y16 = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
            let y = (y16 as f32) * HEIGHT / 65535.0;

            buf.push(x);
            buf.push(y);
        }
    });

    packet_count
}

// JSON文字列からパケット情報を読み取り、共有バッファを更新する関数
#[wasm_bindgen]
pub fn update_packet_buffer_from_json(json_data: &str) -> usize {
    let packets: Vec<JsonPacket> = match serde_json::from_str(json_data) {
        Ok(p) => p,
        Err(_) => return 0,
    };

    PACKET_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();

        let required = packets.len() * 2;
        let current_capacity = buf.capacity();
        if current_capacity < required {
            buf.reserve(required - current_capacity);
        }

        for packet in &packets {
            buf.push(packet.x as f32);
            buf.push(packet.y as f32);
        }

        packets.len()
    })
}

// WasmのメモリインスタンスをJSに返す関数
#[wasm_bindgen]
pub fn get_memory() -> JsValue {
    wasm_bindgen::memory()
}

// =============================================================================
// MESSAGE HANDLING API - WebSocket等からのメッセージ処理
// =============================================================================

// パケットのデータを表す構造体。JSONのシリアライズ/デシリアライズに対応（旧API用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPacket {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}

// JSのconsole.logをRustから使いやすくラップした関数
#[wasm_bindgen]
pub fn console_log(message: &str) {
    log(message);
}

// WebSocketなどで受信したメッセージ（JSONまたは文字列）を処理し、描画を行う関数
#[wasm_bindgen]
pub fn handle_message(message: &str) {
    let msg_size = message.len();
    log(&format!(
        "[Rust/Wasm] Received: {} bytes ({:.2} KB)",
        msg_size,
        msg_size as f64 / 1024.0
    ));

    let start_parse = now();
    if let Ok(packets) = serde_json::from_str::<Vec<JsonPacket>>(message) {
        let parse_time = now() - start_parse;

        log(&format!(
            "[Rust/Wasm] Parsed {} packets in {:.2}ms",
            packets.len(),
            parse_time
        ));

        let start_convert = now();
        let coords: Vec<f32> = packets
            .iter()
            .flat_map(|p| [p.x as f32, p.y as f32])
            .collect();
        let convert_time = now() - start_convert;

        let start_draw = now();
        render_packets_gpu(&coords);
        let draw_time = now() - start_draw;

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

    match serde_json::from_str::<JsonPacket>(message) {
        Ok(packet) => {
            log(&format!(
                "[Rust/Wasm] Parsed single Packet: id={}, x={}, y={}",
                packet.id, packet.x, packet.y
            ));
            let coords = vec![packet.x as f32, packet.y as f32];
            render_packets_gpu(&coords);
        }
        Err(_) => {
            log(&format!("[Rust/Wasm] Plain text: {}", message));
        }
    }
}

// バイナリ形式のパケットデータを受け取り、解析して描画する関数
#[wasm_bindgen]
pub fn handle_binary(data: &[u8]) {
    let packet_count = data.len() / 8;

    let mut coords: Vec<f32> = Vec::with_capacity(packet_count * 2);

    for i in 0..packet_count {
        let offset = i * 8;

        let x16 = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
        let x = (x16 as f32) * WIDTH / 65535.0;

        let y16 = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
        let y = (y16 as f32) * HEIGHT / 65535.0;

        coords.push(x);
        coords.push(y);
    }

    render_packets_gpu(&coords);
}

// =============================================================================
// SIMULATION API - JSからSimulationStateを操作するためのグローバル関数
// =============================================================================

/// シミュレーションを初期化
#[wasm_bindgen]
pub fn create_simulation(max_packets: usize) {
    let sim = SimulationState::new(max_packets);
    SIMULATION_STATE.with(|state| {
        *state.borrow_mut() = Some(sim);
    });
    log(&format!(
        "[Rust/Wasm] Simulation created with {} max packets",
        max_packets
    ));
}

/// シミュレーションにパケット生成予約を追加（座標指定モード）
#[wasm_bindgen]
pub fn simulation_spawn_wave(
    x: f32,
    y: f32,
    target_x: f32,
    target_y: f32,
    count: usize,
    duration_ms: f64,
    base_speed: f32,
    speed_variance: f32,
    packet_type: u32,
    complexity: u8,
) {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.spawn_wave(
                x,
                y,
                target_x,
                target_y,
                count,
                duration_ms,
                base_speed,
                speed_variance,
                packet_type,
                complexity,
            );
        } else {
            log("[Rust/Wasm] Error: Simulation not initialized. Call create_simulation first.");
        }
    });
}

/// シミュレーションにパケット生成予約を追加（ノード指定モード）
#[wasm_bindgen]
pub fn simulation_spawn_wave_to_node(
    x: f32,
    y: f32,
    target_node_idx: i32,
    count: usize,
    duration_ms: f64,
    base_speed: f32,
    speed_variance: f32,
    packet_type: u32,
    complexity: u8,
) {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.spawn_wave_to_node(
                x,
                y,
                target_node_idx,
                count,
                duration_ms,
                base_speed,
                speed_variance,
                packet_type,
                complexity,
            );
        } else {
            log("[Rust/Wasm] Error: Simulation not initialized. Call create_simulation first.");
        }
    });
}

/// ノードを追加
#[wasm_bindgen]
pub fn simulation_add_node(id: u32, x: f32, y: f32, node_type: u32) {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.add_node(id, x, y, node_type);
        } else {
            log("[Rust/Wasm] Error: Simulation not initialized. Call create_simulation first.");
        }
    });
}

/// すべてのノードをクリア
#[wasm_bindgen]
pub fn simulation_clear_nodes() {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.clear_nodes();
        } else {
            log("[Rust/Wasm] Error: Simulation not initialized. Call create_simulation first.");
        }
    });
}

/// ノード数を取得
#[wasm_bindgen]
pub fn simulation_get_node_count() -> usize {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_node_count())
            .unwrap_or(0)
    })
}

/// ノードの位置を更新
#[wasm_bindgen]
pub fn simulation_update_node_position(id: u32, x: f32, y: f32) {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.update_node_position(id, x, y);
        } else {
            log("[Rust/Wasm] Error: Simulation not initialized. Call create_simulation first.");
        }
    });
}

/// テスト用: 指定位置からパケットを生成
#[wasm_bindgen]
pub fn simulation_debug_spawn(x: f32, y: f32, count: usize) {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.debug_spawn(x, y, count);
        } else {
            log("[Rust/Wasm] Error: Simulation not initialized. Call create_simulation first.");
        }
    });
}

/// シミュレーションを1フレーム進める
#[wasm_bindgen]
pub fn simulation_tick(delta_ms: f64) {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.tick(delta_ms);
        }
    });
}

/// アクティブなパケット数を取得
#[wasm_bindgen]
pub fn simulation_get_active_count() -> usize {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_active_count())
            .unwrap_or(0)
    })
}

/// シミュレーションのパケットをWebGPUで描画
#[wasm_bindgen]
pub fn render_simulation_frame() {
    // SimulationStateからアクティブなパケットの座標を取得
    let coords = SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_active_coords())
            .unwrap_or_default()
    });

    // GPUで描画
    render_simulation_frame_internal(&coords);
}
