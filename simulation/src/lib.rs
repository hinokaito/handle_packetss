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
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// =============================================================================
// STAGE CONFIG STRUCTURES - ステージ設定用構造体（Go APIのJSONと対応）
// =============================================================================

/// ステージ全体の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub meta: StageMeta,
    pub map: MapConfig,
    pub waves: Vec<WaveConfig>,
}

/// ステージのメタ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMeta {
    pub title: String,
    pub description: String,
    pub budget: u32,
    pub sla_target: f64,
}

/// マップ設定（固定ノードなど）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    pub fixed_nodes: Vec<FixedNodeConfig>,
}

/// 固定配置されるノード（Gateway等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedNodeConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub x: i32,
    pub y: i32,
}

/// パケット出現パターン（Wave）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveConfig {
    pub time_start_ms: u32,
    pub source_id: String,
    pub count: u32,
    pub duration_ms: u32,
    pub packet_type: String,
    pub speed: f64,
}

/// ロード済みステージの状態（Wave管理用）
#[derive(Debug, Clone)]
pub struct LoadedStage {
    pub config: StageConfig,
    pub node_id_map: HashMap<String, usize>, // "gateway" -> node index
    pub pending_waves: Vec<WaveConfig>,       // まだ発火していないWave
}

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

// ロード済みステージをグローバルに保持
thread_local! {
    static LOADED_STAGE: RefCell<Option<LoadedStage>> = RefCell::new(None);
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

/// シミュレーションのパケットとノードをWebGPUで描画
#[wasm_bindgen]
pub fn render_simulation_frame() {
    // ノードタイプごとの色定義
    // Gateway: 緑, LB: 青, Server: 紫, DB: オレンジ
    let node_colors: [(f32, f32, f32); 4] = [
        (0.14, 0.53, 0.21),  // Gateway: #238636
        (0.12, 0.43, 0.92),  // LB: #1f6feb
        (0.54, 0.34, 0.90),  // Server: #8957e5
        (0.94, 0.53, 0.24),  // DB: #f0883e
    ];
    let node_size = 20.0_f32;
    
    // パケットの色とサイズ
    let packet_color = (1.0_f32, 1.0_f32, 1.0_f32); // 白
    let packet_size = 3.0_f32;

    // エンティティデータを構築: [x, y, r, g, b, size] per entity
    let entity_data = SIMULATION_STATE.with(|state| {
        let mut data: Vec<f32> = Vec::new();
        
        if let Some(sim) = state.borrow().as_ref() {
            // 1. まずノードを追加（大きいので先に描画）
            for i in 0..sim.get_node_count() {
                if let Some((x, y)) = sim.get_node_position_by_index(i) {
                    // ノードタイプを取得（0=Gateway, 1=LB, 2=Server, 3=DB）
                    let node_type = sim.get_node_type_by_index(i).unwrap_or(0) as usize;
                    let color_idx = node_type.min(3); // 0-3の範囲に制限
                    let (r, g, b) = node_colors[color_idx];
                    
                    data.push(x);
                    data.push(y);
                    data.push(r);
                    data.push(g);
                    data.push(b);
                    data.push(node_size);
                }
            }
            
            // 2. 次にパケットを追加
            let coords = sim.get_active_coords();
            for chunk in coords.chunks(2) {
                if chunk.len() == 2 {
                    data.push(chunk[0]); // x
                    data.push(chunk[1]); // y
                    data.push(packet_color.0);
                    data.push(packet_color.1);
                    data.push(packet_color.2);
                    data.push(packet_size);
                }
            }
        }
        
        data
    });

    // GPUで描画
    render_simulation_frame_internal(&entity_data);
}

// =============================================================================
// SIMULATION STATS API - 統計情報取得
// =============================================================================

/// 統計: 生成されたパケット総数を取得
#[wasm_bindgen]
pub fn simulation_get_stats_spawned() -> u32 {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_stats_spawned())
            .unwrap_or(0)
    })
}

/// 統計: 処理完了したパケット数を取得
#[wasm_bindgen]
pub fn simulation_get_stats_processed() -> u32 {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_stats_processed())
            .unwrap_or(0)
    })
}

/// 統計: ドロップしたパケット数を取得
#[wasm_bindgen]
pub fn simulation_get_stats_dropped() -> u32 {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_stats_dropped())
            .unwrap_or(0)
    })
}

/// 現在の経過時間（ミリ秒）を取得
#[wasm_bindgen]
pub fn simulation_get_current_time() -> f64 {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_current_time())
            .unwrap_or(0.0)
    })
}

/// シミュレーション全体をリセット
#[wasm_bindgen]
pub fn simulation_reset() {
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.reset();
        }
    });
}

/// 指定インデックスのノード位置を取得（x, y）、見つからない場合は(-1, -1)
#[wasm_bindgen]
pub fn simulation_get_node_position(index: usize) -> Vec<f32> {
    SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|sim| sim.get_node_position_by_index(index))
            .map(|(x, y)| vec![x, y])
            .unwrap_or_else(|| vec![-1.0, -1.0])
    })
}

// =============================================================================
// STAGE CONFIG API - ステージ設定のロード・管理
// =============================================================================

/// ステージ設定JSONをパースしてロード
/// 固定ノードをシミュレーションに配置し、Wave情報を保持
#[wasm_bindgen]
pub fn load_stage_config(json_str: &str) -> bool {
    // JSONをパース
    let config: StageConfig = match serde_json::from_str(json_str) {
        Ok(c) => c,
        Err(e) => {
            log(&format!("[Rust/Wasm] Failed to parse stage config: {}", e));
            return false;
        }
    };

    log(&format!(
        "[Rust/Wasm] Loading stage: {} (budget={}, sla_target={})",
        config.meta.title, config.meta.budget, config.meta.sla_target
    ));

    // シミュレーションのノードをクリア
    SIMULATION_STATE.with(|state| {
        if let Some(sim) = state.borrow_mut().as_mut() {
            sim.clear_nodes();
        }
    });

    // 固定ノードを配置し、IDマップを構築
    let mut node_id_map: HashMap<String, usize> = HashMap::new();
    
    for (idx, node) in config.map.fixed_nodes.iter().enumerate() {
        let node_type = match node.node_type.to_lowercase().as_str() {
            "gateway" => 0,
            "lb" => 1,
            "server" => 2,
            "db" => 3,
            _ => 0,
        };
        
        SIMULATION_STATE.with(|state| {
            if let Some(sim) = state.borrow_mut().as_mut() {
                sim.add_node(idx as u32, node.x as f32, node.y as f32, node_type);
            }
        });
        
        node_id_map.insert(node.id.clone(), idx);
        log(&format!(
            "[Rust/Wasm] Fixed node added: id={}, type={}, pos=({}, {})",
            node.id, node.node_type, node.x, node.y
        ));
    }

    // Wave情報をコピー（pending_wavesとして保持）
    let pending_waves = config.waves.clone();
    
    log(&format!(
        "[Rust/Wasm] Stage loaded: {} fixed nodes, {} waves",
        config.map.fixed_nodes.len(),
        pending_waves.len()
    ));

    // LoadedStageを保存
    let loaded_stage = LoadedStage {
        config,
        node_id_map,
        pending_waves,
    };

    LOADED_STAGE.with(|stage| {
        *stage.borrow_mut() = Some(loaded_stage);
    });

    true
}

/// ロード済みステージのメタ情報を取得（JSON文字列で返す）
#[wasm_bindgen]
pub fn get_stage_meta() -> Option<String> {
    LOADED_STAGE.with(|stage| {
        stage
            .borrow()
            .as_ref()
            .map(|s| serde_json::to_string(&s.config.meta).unwrap_or_default())
    })
}

/// ロード済みステージの予算を取得
#[wasm_bindgen]
pub fn get_stage_budget() -> u32 {
    LOADED_STAGE.with(|stage| {
        stage
            .borrow()
            .as_ref()
            .map(|s| s.config.meta.budget)
            .unwrap_or(0)
    })
}

/// ロード済みステージのSLAターゲットを取得
#[wasm_bindgen]
pub fn get_stage_sla_target() -> f64 {
    LOADED_STAGE.with(|stage| {
        stage
            .borrow()
            .as_ref()
            .map(|s| s.config.meta.sla_target)
            .unwrap_or(0.0)
    })
}

/// 指定した時刻までのWaveを発火させる
/// シミュレーション開始後、current_timeに応じて呼び出す
#[wasm_bindgen]
pub fn trigger_waves_until(current_time_ms: u32) {
    // pending_wavesから発火すべきWaveを取得
    let waves_to_trigger: Vec<(WaveConfig, Option<usize>)> = LOADED_STAGE.with(|stage| {
        let mut stage_ref = stage.borrow_mut();
        if let Some(loaded) = stage_ref.as_mut() {
            let mut to_trigger = Vec::new();
            let mut remaining = Vec::new();
            
            for wave in loaded.pending_waves.drain(..) {
                if wave.time_start_ms <= current_time_ms {
                    // source_idからノードインデックスを解決
                    let source_idx = loaded.node_id_map.get(&wave.source_id).copied();
                    to_trigger.push((wave, source_idx));
                } else {
                    remaining.push(wave);
                }
            }
            
            loaded.pending_waves = remaining;
            to_trigger
        } else {
            Vec::new()
        }
    });

    // Waveを発火
    for (wave, source_idx) in waves_to_trigger {
        if let Some(idx) = source_idx {
            // ソースノードの位置を取得
            let source_pos = SIMULATION_STATE.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .and_then(|sim| sim.get_node_position_by_index(idx))
            });

            if let Some((x, y)) = source_pos {
                let packet_type = match wave.packet_type.to_uppercase().as_str() {
                    "NORMAL" => 0,
                    "SYN_FLOOD" | "SYNFLOOD" => 1,
                    "HEAVY_TASK" | "HEAVYTASK" => 2,
                    "KILLER" => 3,
                    _ => 0,
                };

                SIMULATION_STATE.with(|state| {
                    if let Some(sim) = state.borrow_mut().as_mut() {
                        // Gatewayからの場合は次のノード（LB=1）へ向かう
                        sim.spawn_wave_to_node(
                            x,
                            y,
                            (idx + 1) as i32, // 次のノードへ（簡易実装）
                            wave.count as usize,
                            wave.duration_ms as f64,
                            wave.speed as f32,
                            1.0, // speed_variance
                            packet_type,
                            10,  // complexity
                        );
                    }
                });

                log(&format!(
                    "[Rust/Wasm] Wave triggered: {} packets from {} at t={}ms",
                    wave.count, wave.source_id, wave.time_start_ms
                ));
            }
        } else {
            log(&format!(
                "[Rust/Wasm] Warning: source_id '{}' not found in node_id_map",
                wave.source_id
            ));
        }
    }
}

/// 残りのWave数を取得
#[wasm_bindgen]
pub fn get_pending_wave_count() -> usize {
    LOADED_STAGE.with(|stage| {
        stage
            .borrow()
            .as_ref()
            .map(|s| s.pending_waves.len())
            .unwrap_or(0)
    })
}

/// ステージをリセット（Waveを再ロード）
#[wasm_bindgen]
pub fn reset_stage_waves() {
    LOADED_STAGE.with(|stage| {
        let mut stage_ref = stage.borrow_mut();
        if let Some(loaded) = stage_ref.as_mut() {
            loaded.pending_waves = loaded.config.waves.clone();
            log(&format!(
                "[Rust/Wasm] Stage waves reset: {} waves pending",
                loaded.pending_waves.len()
            ));
        }
    });
}
