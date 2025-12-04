// =============================================================================
// SIMULATION ENGINE - パケット生成・シミュレーションロジック担当
// =============================================================================

use wasm_bindgen::prelude::*;

// キャンバスサイズ定数
pub const WIDTH: f32 = 1920.0;
pub const HEIGHT: f32 = 1080.0;

// JS側の関数（console.log）をRustで使うための宣言
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// JavaScriptのMath.random()を使用
fn js_random() -> f32 {
    js_sys::Math::random() as f32
}

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

    /// ノードの位置を更新（JSから呼び出し）
    pub fn update_node_position(&mut self, id: u32, x: f32, y: f32) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.x = x;
            node.y = y;
            log(&format!(
                "[Rust/Wasm] Node position updated: id={}, pos=({}, {})",
                id, x, y
            ));
        } else {
            log(&format!(
                "[Rust/Wasm] Warning: Node with id={} not found for position update",
                id
            ));
        }
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
