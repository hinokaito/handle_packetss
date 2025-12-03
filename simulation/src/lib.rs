use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::util::DeviceExt;
use wgpu::*;

// =============================================================================
// WEBGPU RENDERER
// =============================================================================
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
// シェーダーに時間を渡すためのユニフォームバッファ構造体。アライメント調整用のパディングを含む
struct TimeUniform {
    time: f32,
    _padding: [f32; 7],
}

// WebGPUのデバイス、キュー、パイプラインなど、描画に必要なリソースをまとめて管理する構造体
struct GpuRenderer {
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    packet_buffer: Buffer, // 描画したいパケットの座標データをGPUメモリ上に保持するための領域
    packet_count: u32,     // 現在バッファに含まれている、あるいは描画するべきパケットの数を管理する
    surface: Surface<'static>, // 画面(この場合はHTMLの<canvas>要素)への描画領域を表す。レンダリング結果を最終的にユーザーに見せるための窓口
    surface_config: SurfaceConfiguration, // サーフェスの設定情報を保持する。ウィンドウサイズが変わった際など、surfaceを再設定するために必要
    canvas_width: u32,

    canvas_height: u32,
    time_buffer: Buffer,
    time_bind_group: BindGroup,
}

// 初期化したGpuRendererインスタンスをプログラムのどこからでもアクセスできるように保持しておく場所。
// [LEARN]thread_local!マクロを使用することで、「スレッドごとに1つ(=Wasm環境全体で実質1つ)の書き換え可能な永続データ」を安全に管理できる
// GPUレンダラー置き場を作っておき、最初は空にする。
// init_gpu関数で初期化が成功したら、ここにレンダラーを格納し、描画関数でここからレンダーを取り出して使う
thread_local! {
    // GpuRendererのインスタンスをスレッドローカル（Wasmでは実質グローバル）に保持するための変数
    static GPU_RENDERER: RefCell<Option<GpuRenderer>> = RefCell::new(None);
}

// JavaScriptからRustへ大量のデータを渡す際や、計算結果を一時的に保持するための「使いまわし可能なメモリ領域」
// 毎回新しいメモリを確保して開放するのは遅いため、このPACKET_BUFFERという「常駐する領域」を1つ用意しておく
thread_local! {
    // JSとRust間でデータをやり取りするための一時的な共有メモリバッファ
    static PACKET_BUFFER: RefCell<Vec<f32>> = RefCell::new(Vec::new());
}

// シミュレーション状態をグローバルに保持（JSから複数回アクセスするため）
thread_local! {
    static SIMULATION_STATE: RefCell<Option<SimulationState>> = RefCell::new(None);
}

const WIDTH: f32 = 1920.0;
const HEIGHT: f32 = 1080.0;

// =============================================================================
// SIMULATION ENGINE - パケット生成・シミュレーション
// =============================================================================

/// パケットタイプの列挙型
#[wasm_bindgen]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PacketType {
    Normal = 0,
    SynFlood = 1,
    HeavyTask = 2,
    Killer = 3,
}

/// ノードタイプの列挙型
#[wasm_bindgen]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeType {
    Gateway = 0, // パケットの入口
    LB = 1,      // ロードバランサー
    Server = 2,  // アプリケーションサーバー
    DB = 3,      // データベース
}

/// ノード構造体（目的地となるオブジェクト）
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub x: f32,
    pub y: f32,
    pub id: u32,        // ユニークID（JS側での管理用）
    pub node_type: u32, // NodeType as u32
}

/// シミュレーション用パケット構造体
/// WebGPUに渡すため#[repr(C)]でメモリレイアウトを固定
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Packet {
    pub x: f32,
    pub y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub active: u32,          // 0: inactive, 1: active
    pub packet_type: u32,     // PacketType as u32
    pub complexity: u8,       // 処理の重さ係数
    pub _padding: [u8; 3],    // アライメント用パディング
    pub target_node_idx: i32, // 目標ノードのインデックス (-1 = 宛先なし)
    pub speed: f32,           // 移動速度（ピクセル/フレーム）
}

impl Default for Packet {
    fn default() -> Self {
        Packet {
            x: 0.0,
            y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            active: 0,
            packet_type: 0,
            complexity: 0,
            _padding: [0; 3],
            target_node_idx: -1, // 宛先なし
            speed: 3.0,          // デフォルト速度
        }
    }
}

/// パケット生成予約タスク
/// spawn_waveで登録し、tick()で徐々に生成する
#[derive(Clone, Debug)]
struct SpawnTask {
    x: f32,
    y: f32,
    target_x: f32,
    target_y: f32,
    target_node_idx: i32, // ターゲットノードのインデックス (-1 = 座標指定モード)
    total_count: usize,   // 生成する総数
    spawned_count: usize, // 生成済みの数
    duration_ms: f64,     // 何ミリ秒かけて放出するか
    base_speed: f32,
    speed_variance: f32,
    packet_type: u32,
    complexity: u8,
    start_time: f64, // タスク開始時刻（performance.now()）
}

/// シミュレーション状態を管理する構造体
#[wasm_bindgen]
pub struct SimulationState {
    packets: Vec<Packet>,
    nodes: Vec<Node>, // ノード（目的地）のリスト
    max_packets: usize,
    spawn_queue: Vec<SpawnTask>,
    current_time: f64,
}

#[wasm_bindgen]
impl SimulationState {
    /// 新しいSimulationStateを作成
    /// max_packets: 同時に存在できるパケットの最大数
    #[wasm_bindgen(constructor)]
    pub fn new(max_packets: usize) -> SimulationState {
        let packets = vec![Packet::default(); max_packets];
        log(&format!(
            "[Rust/Wasm] SimulationState created with {} packet slots",
            max_packets
        ));
        SimulationState {
            packets,
            nodes: Vec::new(), // ノードリスト初期化
            max_packets,
            spawn_queue: Vec::new(),
            current_time: 0.0,
        }
    }

    /// ノードを追加（JSから呼び出し）
    pub fn add_node(&mut self, id: u32, x: f32, y: f32, node_type: u32) {
        self.nodes.push(Node {
            x,
            y,
            id,
            node_type,
        });
        log(&format!(
            "[Rust/Wasm] Node added: id={}, pos=({}, {}), type={}",
            id, x, y, node_type
        ));
    }

    /// すべてのノードをクリア
    pub fn clear_nodes(&mut self) {
        self.nodes.clear();
        log("[Rust/Wasm] All nodes cleared");
    }

    /// ノード数を取得
    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// パケット生成予約を追加（座標指定モード）
    /// Goから送られてくる生成情報を受け取り、spawn_queueに追加する
    pub fn spawn_wave(
        &mut self,
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
        let task = SpawnTask {
            x,
            y,
            target_x,
            target_y,
            target_node_idx: -1, // 座標指定モード
            total_count: count,
            spawned_count: 0,
            duration_ms,
            base_speed,
            speed_variance,
            packet_type,
            complexity,
            start_time: self.current_time,
        };

        log(&format!(
            "[Rust/Wasm] spawn_wave: {} packets from ({}, {}) to ({}, {}), duration={}ms, speed={} ± {}",
            count, x, y, target_x, target_y, duration_ms, base_speed, speed_variance
        ));

        self.spawn_queue.push(task);
    }

    /// パケット生成予約を追加（ノード指定モード）
    /// パケットは指定されたノードに向かって移動する
    pub fn spawn_wave_to_node(
        &mut self,
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
        let task = SpawnTask {
            x,
            y,
            target_x: 0.0, // 使用しない
            target_y: 0.0, // 使用しない
            target_node_idx,
            total_count: count,
            spawned_count: 0,
            duration_ms,
            base_speed,
            speed_variance,
            packet_type,
            complexity,
            start_time: self.current_time,
        };

        log(&format!(
            "[Rust/Wasm] spawn_wave_to_node: {} packets from ({}, {}) to node[{}], duration={}ms, speed={} ± {}",
            count, x, y, target_node_idx, duration_ms, base_speed, speed_variance
        ));

        self.spawn_queue.push(task);
    }

    /// テスト用の簡易スポーン関数
    /// 指定位置からランダムな方向にパケットを生成
    pub fn debug_spawn(&mut self, x: f32, y: f32, count: usize) {
        let mut spawned = 0;
        for packet in self.packets.iter_mut() {
            if packet.active == 0 {
                packet.active = 1;
                packet.x = x;
                packet.y = y;
                // ランダムな方向に散らばらせる
                packet.velocity_x = (js_random() - 0.5) * 4.0;
                packet.velocity_y = (js_random() - 0.5) * 4.0;
                packet.packet_type = PacketType::Normal as u32;
                packet.complexity = 10;

                spawned += 1;
                if spawned >= count {
                    break;
                }
            }
        }
        log(&format!(
            "[Rust/Wasm] debug_spawn: spawned {} packets at ({}, {})",
            spawned, x, y
        ));
    }

    /// 毎フレーム呼び出す更新関数
    /// delta_ms: 前フレームからの経過時間（ミリ秒）
    pub fn tick(&mut self, delta_ms: f64) {
        self.current_time += delta_ms;

        // 1. spawn_queueを処理: 予約に基づいてパケットを生成
        self.process_spawn_queue();

        // 2. アクティブなパケットを更新
        self.update_packets(delta_ms);
    }

    /// アクティブなパケット数を返す
    pub fn get_active_count(&self) -> usize {
        self.packets.iter().filter(|p| p.active == 1).count()
    }

    /// WebGPU描画用にパケットメモリのポインタを返す
    pub fn get_packets_ptr(&self) -> *const Packet {
        self.packets.as_ptr()
    }

    /// 最大パケット数を返す
    pub fn get_max_packets(&self) -> usize {
        self.max_packets
    }
}

// SimulationStateの内部実装（#[wasm_bindgen]なし）
impl SimulationState {
    /// spawn_queueを処理し、適切な数のパケットを生成
    fn process_spawn_queue(&mut self) {
        let current_time = self.current_time;

        // 完了したタスクを追跡
        let mut completed_indices = Vec::new();

        for (idx, task) in self.spawn_queue.iter_mut().enumerate() {
            let elapsed = current_time - task.start_time;

            // このフレームで生成すべき数を計算
            let target_spawned = if task.duration_ms <= 0.0 {
                // duration_ms が 0 なら即時全生成
                task.total_count
            } else {
                // 経過時間に応じて線形に生成
                let progress = (elapsed / task.duration_ms).min(1.0);
                (task.total_count as f64 * progress) as usize
            };

            let to_spawn = target_spawned.saturating_sub(task.spawned_count);

            if to_spawn > 0 {
                let mut actually_spawned = 0;
                for packet in self.packets.iter_mut() {
                    if packet.active == 0 && actually_spawned < to_spawn {
                        // パケットを生成
                        packet.active = 1;
                        packet.x = task.x;
                        packet.y = task.y;

                        // 速度にばらつきを加える
                        let speed =
                            task.base_speed + (js_random() - 0.5) * 2.0 * task.speed_variance;
                        packet.speed = speed;

                        // ノード指定モードかチェック
                        if task.target_node_idx >= 0 {
                            // ノードターゲットモード: パケットにターゲットノードを設定
                            packet.target_node_idx = task.target_node_idx;
                            // velocity は使わない（update_packetsでベクトル計算）
                            packet.velocity_x = 0.0;
                            packet.velocity_y = 0.0;
                        } else {
                            // 座標指定モード（従来の動作）
                            packet.target_node_idx = -1;
                            let dx = task.target_x - task.x;
                            let dy = task.target_y - task.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            let (dir_x, dir_y) = if dist > 0.0 {
                                (dx / dist, dy / dist)
                            } else {
                                (1.0, 0.0)
                            };
                            packet.velocity_x = dir_x * speed;
                            packet.velocity_y = dir_y * speed;
                        }

                        packet.packet_type = task.packet_type;
                        packet.complexity = task.complexity;

                        actually_spawned += 1;
                    }
                }

                task.spawned_count += actually_spawned;
            }

            // タスク完了チェック
            if task.spawned_count >= task.total_count {
                completed_indices.push(idx);
            }
        }

        // 完了したタスクを削除（逆順で削除してインデックスがずれないように）
        for idx in completed_indices.into_iter().rev() {
            self.spawn_queue.remove(idx);
        }
    }

    /// アクティブなパケットの位置を更新
    fn update_packets(&mut self, _delta_ms: f64) {
        // 到達したパケットのインデックスを収集
        let mut arrived_packets: Vec<usize> = Vec::new();

        // まずパケットの移動処理（不変借用でノードを参照）
        for (idx, packet) in self.packets.iter_mut().enumerate() {
            if packet.active == 1 {
                // ノードターゲットモード
                if packet.target_node_idx >= 0
                    && (packet.target_node_idx as usize) < self.nodes.len()
                {
                    let target = &self.nodes[packet.target_node_idx as usize];

                    // ベクトル計算（目的地 - 現在地）
                    let dx = target.x - packet.x;
                    let dy = target.y - packet.y;

                    // 距離計算
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt();

                    // 到達判定（半径5.0以内なら到着）
                    if dist < 5.0 {
                        // 到達！→ 後で処理
                        arrived_packets.push(idx);
                    } else {
                        // 正規化して速度を掛けて移動
                        if dist > 0.0 {
                            packet.x += (dx / dist) * packet.speed;
                            packet.y += (dy / dist) * packet.speed;
                        }
                    }
                } else if packet.target_node_idx == -1 {
                    // 座標指定モード（従来のvelocity使用）
                    packet.x += packet.velocity_x;
                    packet.y += packet.velocity_y;

                    // 画面外に出たら非アクティブに
                    if packet.x < -50.0
                        || packet.x > WIDTH + 50.0
                        || packet.y < -50.0
                        || packet.y > HEIGHT + 50.0
                    {
                        packet.active = 0;
                    }
                } else {
                    // ターゲットがないか無効ならその場で消滅
                    packet.active = 0;
                }
            }
        }

        // 到達したパケットの処理（ルーティング）
        for packet_idx in arrived_packets {
            self.handle_packet_arrival(packet_idx);
        }
    }

    /// パケットがターゲットノードに到達したときの処理
    fn handle_packet_arrival(&mut self, packet_idx: usize) {
        // Rustの借用ルール回避のため、必要な情報をコピーして取得
        let (target_node_idx, _packet_type) = {
            let p = &self.packets[packet_idx];
            (p.target_node_idx, p.packet_type)
        };

        // ターゲットが存在しないなら終了
        if target_node_idx < 0 {
            self.packets[packet_idx].active = 0;
            return;
        }

        // 到達したノードの情報を取得
        let node_type = self.nodes[target_node_idx as usize].node_type;
        let current_node_pos = (
            self.nodes[target_node_idx as usize].x,
            self.nodes[target_node_idx as usize].y,
        );

        match node_type {
            0 => {
                // Type 0: Gateway (入口)
                // Gateway -> LBへルーティング
                if let Some(next_idx) = self.find_next_node_by_type(1) {
                    let p = &mut self.packets[packet_idx];
                    p.target_node_idx = next_idx as i32;
                    p.x = current_node_pos.0;
                    p.y = current_node_pos.1;
                } else {
                    self.packets[packet_idx].active = 0;
                }
            }
            1 => {
                // Type 1: Load Balancer (LB)
                // LB -> Serverへルーティング（ラウンドロビン的に分散）
                if let Some(next_idx) = self.find_next_server_target() {
                    let p = &mut self.packets[packet_idx];
                    p.target_node_idx = next_idx as i32;
                    p.x = current_node_pos.0;
                    p.y = current_node_pos.1;
                } else {
                    self.packets[packet_idx].active = 0;
                }
            }
            2 => {
                // Type 2: Server
                // Server -> DBへルーティング
                if let Some(next_idx) = self.find_next_node_by_type(3) {
                    let p = &mut self.packets[packet_idx];
                    p.target_node_idx = next_idx as i32;
                    p.x = current_node_pos.0;
                    p.y = current_node_pos.1;
                } else {
                    // DBがない場合は処理完了
                    self.packets[packet_idx].active = 0;
                }
            }
            3 => {
                // Type 3: DB
                // DB到達 = リクエスト処理完了
                self.packets[packet_idx].active = 0;
            }
            _ => {
                // その他
                self.packets[packet_idx].active = 0;
            }
        }
    }

    /// 指定タイプのノードを検索して返す
    fn find_next_node_by_type(&self, node_type: u32) -> Option<usize> {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.node_type == node_type {
                return Some(i);
            }
        }
        None
    }

    /// ロードバランシング: Serverノードをラウンドロビン的に選択
    /// 複数のServerがある場合、ランダムに選択
    fn find_next_server_target(&self) -> Option<usize> {
        // node_type == 2 (Server) のノードを収集
        let servers: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.node_type == 2)
            .map(|(i, _)| i)
            .collect();

        if servers.is_empty() {
            None
        } else {
            // ランダムに1つ選択
            let random_idx = (js_random() * servers.len() as f32) as usize;
            Some(servers[random_idx.min(servers.len() - 1)])
        }
    }

    /// アクティブなパケットの座標をf32配列として抽出（描画用）
    pub fn get_active_coords(&self) -> Vec<f32> {
        let mut coords = Vec::new();
        for packet in &self.packets {
            if packet.active == 1 {
                coords.push(packet.x);
                coords.push(packet.y);
            }
        }
        coords
    }
}

// JavaScriptのMath.random()を使用
fn js_random() -> f32 {
    js_sys::Math::random() as f32
}

// WGSL言語で記述された頂点シェーダーとフラグメントシェーダーのソースコード（外部ファイルから読み込み）
const SHADER_SOURCE: &str = include_str!("shader.wgsl");

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

// 実際のWebGPU初期化処理を行う非同期関数。デバイスやパイプラインの作成を行う
async fn init_gpu_internal(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no global Window exists"))?;

    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no Document exists"))?;

    let canvas = document
        .get_element_by_id(canvas_id)
        .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
        .ok_or_else(|| JsValue::from_str("canvas element not found"))?;

    let canvas_width = canvas.width();
    let canvas_height = canvas.height();

    log(&format!(
        "[Rust/Wasm] Initializing WebGPU for canvas {}x{}",
        canvas_width, canvas_height
    ));

    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::BROWSER_WEBGPU | Backends::GL,
        ..Default::default()
    });

    let surface = instance
        .create_surface(SurfaceTarget::Canvas(canvas))
        .expect("Failed to create surface");

    let adapter = match instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
    {
        Some(adapter) => adapter,
        None => {
            log("[Rust/Wasm] Failed to get WebGPU adapter");
            return Err(JsValue::from_str("Failed to get WebGPU adapter"));
        }
    };

    let (device, queue) = match adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: MemoryHints::default(),
            },
            None,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let err_msg = format!("Failed to get WebGPU device: {:?}", e);
            log(&err_msg);
            return Err(JsValue::from_str(&err_msg));
        }
    };

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| matches!(f, TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm))
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let surface_config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: canvas_width,
        height: canvas_height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &surface_config);

    let time_uniform = TimeUniform {
        time: 0.0,
        _padding: [0.0; 7],
    };

    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Time Buffer"),
        contents: bytemuck::cast_slice(&[time_uniform]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let time_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("time_bind_group_layout"),
    });

    let time_bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &time_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: time_buffer.as_entire_binding(),
        }],
        label: Some("time_bind_group"),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&time_bind_group_layout],
        push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Packet Shader"),
        source: ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });

    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Packet Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[VertexBufferLayout {
                array_stride: std::mem::size_of::<f32>() as u64 * 2,
                step_mode: VertexStepMode::Instance,
                attributes: &[VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                }],
            }],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: surface_config.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    let max_packets = 100_000;
    let packet_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Packet Buffer"),
        size: (max_packets * 2 * std::mem::size_of::<f32>()) as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let renderer = GpuRenderer {
        device,
        queue,
        render_pipeline,
        packet_buffer,
        packet_count: 0,
        surface,
        surface_config,
        canvas_width,
        canvas_height,
        time_buffer,
        time_bind_group,
    };

    GPU_RENDERER.with(|r| {
        *r.borrow_mut() = Some(renderer);
    });

    log("[Rust/Wasm] WebGPU initialized successfully!");
    Ok(())
}

// 一度に描画できるパケットの最大数
const MAX_PACKETS: usize = 100_000;

// 与えられた座標データを使ってGPUでパケットを描画する関数
fn render_packets_gpu(coords: &[f32]) {
    GPU_RENDERER.with(|renderer_ref| {
        let mut renderer_opt = renderer_ref.borrow_mut();
        if let Some(renderer) = renderer_opt.as_mut() {
            let total_packets = coords.len() / 2;
            if total_packets == 0 {
                log("[Rust/Wasm] No packets to render");
                return;
            }

            let packet_count = total_packets.min(MAX_PACKETS);
            let coords_to_render = &coords[0..(packet_count * 2)];

            if total_packets > MAX_PACKETS {
                log(&format!(
                    "[Rust/Wasm] Warning: {} packets received, rendering only {} (buffer limit)",
                    total_packets, packet_count
                ));
            } else {
                log(&format!("[Rust/Wasm] Rendering {} packets", packet_count));
            }

            renderer.queue.write_buffer(
                &renderer.packet_buffer,
                0,
                bytemuck::cast_slice(coords_to_render),
            );

            let current_time = (now() / 1000.0) as f32;
            let time_data = TimeUniform {
                time: current_time,
                _padding: [0.0; 7],
            };
            renderer.queue.write_buffer(
                &renderer.time_buffer,
                0,
                bytemuck::cast_slice(&[time_data]),
            );

            let surface_texture = match renderer.surface.get_current_texture() {
                Ok(texture) => texture,
                Err(e) => {
                    log(&format!(
                        "[Rust/Wasm] Failed to get surface texture: {:?}",
                        e
                    ));
                    return;
                }
            };

            let view = surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default());

            {
                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.050980392156862744, // #0d1117
                                    g: 0.050980392156862744,
                                    b: 0.09019607843137255,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    render_pass.set_pipeline(&renderer.render_pipeline);
                    render_pass.set_bind_group(0, &renderer.time_bind_group, &[]);
                    let buffer_size = (packet_count * 2 * std::mem::size_of::<f32>()) as u64;
                    render_pass.set_vertex_buffer(0, renderer.packet_buffer.slice(0..buffer_size));
                    render_pass.draw(0..4, 0..packet_count as u32);
                }

                renderer.queue.submit(Some(encoder.finish()));
            }

            surface_texture.present();
            renderer.packet_count = packet_count as u32;
            log(&format!(
                "[Rust/Wasm] Rendered {} packets successfully",
                packet_count
            ));
        } else {
            log("[Rust/Wasm] GPU renderer not initialized");
        }
    });
}

// アニメーションフレームごとに呼び出され、画面を再描画する関数
#[wasm_bindgen]
pub fn render_frame() {
    GPU_RENDERER.with(|renderer_ref| {
        let mut renderer_opt = renderer_ref.borrow_mut();
        if let Some(renderer) = renderer_opt.as_mut() {
            let packet_count = renderer.packet_count as usize;
            if packet_count == 0 {
                return;
            }

            let current_time = (now() / 1000.0) as f32;
            let time_data = TimeUniform {
                time: current_time,
                _padding: [0.0; 7],
            };
            renderer.queue.write_buffer(
                &renderer.time_buffer,
                0,
                bytemuck::cast_slice(&[time_data]),
            );

            let surface_texture = match renderer.surface.get_current_texture() {
                Ok(texture) => texture,
                Err(_) => return,
            };

            let view = surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default());

            {
                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.050980392156862744,
                                    g: 0.050980392156862744,
                                    b: 0.09019607843137255,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    render_pass.set_pipeline(&renderer.render_pipeline);
                    render_pass.set_bind_group(0, &renderer.time_bind_group, &[]);
                    let buffer_size = (packet_count * 2 * std::mem::size_of::<f32>()) as u64;
                    render_pass.set_vertex_buffer(0, renderer.packet_buffer.slice(0..buffer_size));
                    render_pass.draw(0..4, 0..packet_count as u32);
                }

                renderer.queue.submit(Some(encoder.finish()));
            }

            surface_texture.present();
        }
    });
}

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

// パケットのデータを表す構造体。JSONのシリアライズ/デシリアライズに対応（旧API用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPacket {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}

#[wasm_bindgen]
// JS側の関数（console.log, performance.now）をRustで使うための宣言
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
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

// Wasmモジュール読み込み時に自動実行されるエントリーポイント
#[wasm_bindgen(start)]
pub fn main() {
    log("[Rust/Wasm] Module initialized!");
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
    // 1. SimulationStateからアクティブなパケットの座標を取得
    let coords = SIMULATION_STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|sim| sim.get_active_coords())
            .unwrap_or_default()
    });

    if coords.is_empty() {
        // パケットがない場合は画面クリアのみ
        GPU_RENDERER.with(|renderer_ref| {
            let mut renderer_opt = renderer_ref.borrow_mut();
            if let Some(renderer) = renderer_opt.as_mut() {
                renderer.packet_count = 0;

                let surface_texture = match renderer.surface.get_current_texture() {
                    Ok(texture) => texture,
                    Err(_) => return,
                };

                let view = surface_texture
                    .texture
                    .create_view(&TextureViewDescriptor::default());

                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Clear Encoder"),
                        });

                {
                    let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Clear Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.050980392156862744,
                                    g: 0.050980392156862744,
                                    b: 0.09019607843137255,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                }

                renderer.queue.submit(Some(encoder.finish()));
                surface_texture.present();
            }
        });
        return;
    }

    // 2. GPUで描画
    GPU_RENDERER.with(|renderer_ref| {
        let mut renderer_opt = renderer_ref.borrow_mut();
        if let Some(renderer) = renderer_opt.as_mut() {
            let total_packets = coords.len() / 2;
            let packet_count = total_packets.min(MAX_PACKETS);
            let coords_to_render = &coords[0..(packet_count * 2)];

            renderer.queue.write_buffer(
                &renderer.packet_buffer,
                0,
                bytemuck::cast_slice(coords_to_render),
            );

            let current_time = (now() / 1000.0) as f32;
            let time_data = TimeUniform {
                time: current_time,
                _padding: [0.0; 7],
            };
            renderer.queue.write_buffer(
                &renderer.time_buffer,
                0,
                bytemuck::cast_slice(&[time_data]),
            );

            let surface_texture = match renderer.surface.get_current_texture() {
                Ok(texture) => texture,
                Err(_) => return,
            };

            let view = surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default());

            {
                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Simulation Render Encoder"),
                        });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Simulation Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.050980392156862744,
                                    g: 0.050980392156862744,
                                    b: 0.09019607843137255,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    render_pass.set_pipeline(&renderer.render_pipeline);
                    render_pass.set_bind_group(0, &renderer.time_bind_group, &[]);
                    let buffer_size = (packet_count * 2 * std::mem::size_of::<f32>()) as u64;
                    render_pass.set_vertex_buffer(0, renderer.packet_buffer.slice(0..buffer_size));
                    render_pass.draw(0..4, 0..packet_count as u32);
                }

                renderer.queue.submit(Some(encoder.finish()));
            }

            surface_texture.present();
            renderer.packet_count = packet_count as u32;
        }
    });
}
