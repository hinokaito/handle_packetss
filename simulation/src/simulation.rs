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

/// ノードスペック（グレードごとの性能）
#[derive(Clone, Copy, Debug, Default)]
pub struct NodeSpec {
    pub max_concurrent: u32,    // 同時処理可能数
    pub process_time_ms: f64,   // 1パケットの処理時間（ミリ秒）
    pub queue_capacity: u32,    // 待機キュー容量
    pub cost: u32,              // 配置コスト
}

/// ノード構造体（目的地となるオブジェクト）
#[derive(Clone, Debug)]
pub struct Node {
    pub x: f32,
    pub y: f32,
    pub id: u32,        // ユニークID（JS側での管理用）
    pub node_type: u32, // NodeType as u32
    pub spec: NodeSpec, // 性能スペック
    // 状態（動的）
    pub processing_packets: Vec<ProcessingPacket>, // 処理中のパケット
    pub queue: Vec<QueuedPacket>,                  // 待機キュー
    pub total_processed: u32,                       // 処理完了数
    pub total_dropped: u32,                         // ドロップ数
}

/// 処理中のパケット情報
#[derive(Clone, Debug)]
pub struct ProcessingPacket {
    pub packet_idx: usize,      // パケットのインデックス
    pub remaining_time_ms: f64, // 残り処理時間
}

/// キュー内で待機中のパケット
#[derive(Clone, Debug)]
pub struct QueuedPacket {
    pub packet_idx: usize,
}

impl Node {
    pub fn new(id: u32, x: f32, y: f32, node_type: u32) -> Self {
        // デフォルトスペック（node_typeに応じて設定）
        let spec = match node_type {
            0 => NodeSpec { // Gateway: 無制限（通過のみ）
                max_concurrent: 10000,
                process_time_ms: 0.0,
                queue_capacity: 10000,
                cost: 0,
            },
            1 => NodeSpec { // LB: 高スループット
                max_concurrent: 100,
                process_time_ms: 10.0,
                queue_capacity: 500,
                cost: 100,
            },
            2 => NodeSpec { // Server: Medium相当
                max_concurrent: 20,
                process_time_ms: 50.0,
                queue_capacity: 50,
                cost: 150,
            },
            3 => NodeSpec { // DB: 低スループット
                max_concurrent: 10,
                process_time_ms: 30.0,
                queue_capacity: 100,
                cost: 200,
            },
            _ => NodeSpec::default(),
        };

        Node {
            x,
            y,
            id,
            node_type,
            spec,
            processing_packets: Vec::new(),
            queue: Vec::new(),
            total_processed: 0,
            total_dropped: 0,
        }
    }

    /// 現在の処理中パケット数
    pub fn current_load(&self) -> u32 {
        self.processing_packets.len() as u32
    }

    /// キュー内パケット数
    pub fn queue_size(&self) -> u32 {
        self.queue.len() as u32
    }

    /// 負荷率（0.0 - 1.0+）
    pub fn load_rate(&self) -> f32 {
        if self.spec.max_concurrent == 0 {
            return 0.0;
        }
        self.current_load() as f32 / self.spec.max_concurrent as f32
    }
}

/// パケット状態
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PacketState {
    Moving = 0,     // 移動中
    Processing = 1, // ノードで処理中
    Queued = 2,     // ノードのキューで待機中
}

/// シミュレーション用パケット構造体
#[derive(Clone, Copy, Debug)]
pub struct Packet {
    pub x: f32,
    pub y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub active: u32,          // 0: inactive, 1: active
    pub packet_type: u32,     // PacketType as u32
    pub complexity: u8,       // 処理の重さ係数
    pub target_node_idx: i32, // 目標ノードのインデックス (-1 = 宛先なし)
    pub speed: f32,           // 移動速度（ピクセル/フレーム）
    pub state: PacketState,   // 現在の状態
    pub current_node_idx: i32, // 現在いるノードのインデックス (-1 = 移動中)
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
            target_node_idx: -1,
            speed: 3.0,
            state: PacketState::Moving,
            current_node_idx: -1,
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

/// シミュレーション統計
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Default)]
pub struct SimulationStats {
    pub packets_spawned: u32,    // 生成されたパケット総数
    pub packets_processed: u32,  // 正常に処理完了したパケット数（DB到達）
    pub packets_dropped: u32,    // ドロップ/失敗したパケット数
    pub packets_in_flight: u32,  // 現在処理中のパケット数
}

/// シミュレーション状態を管理する構造体
#[wasm_bindgen]
pub struct SimulationState {
    packets: Vec<Packet>,
    nodes: Vec<Node>, // ノード（目的地）のリスト
    max_packets: usize,
    spawn_queue: Vec<SpawnTask>,
    current_time: f64,
    stats: SimulationStats, // 統計情報
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
            stats: SimulationStats::default(),
        }
    }

    /// ノードを追加（JSから呼び出し）
    pub fn add_node(&mut self, id: u32, x: f32, y: f32, node_type: u32) {
        let node = Node::new(id, x, y, node_type);
        log(&format!(
            "[Rust/Wasm] Node added: id={}, pos=({}, {}), type={}, max_concurrent={}, process_time={}ms",
            id, x, y, node_type, node.spec.max_concurrent, node.spec.process_time_ms
        ));
        self.nodes.push(node);
    }

    /// スペック付きでノードを追加
    pub fn add_node_with_spec(
        &mut self,
        id: u32,
        x: f32,
        y: f32,
        node_type: u32,
        max_concurrent: u32,
        process_time_ms: f64,
        queue_capacity: u32,
        cost: u32,
    ) {
        let mut node = Node::new(id, x, y, node_type);
        node.spec = NodeSpec {
            max_concurrent,
            process_time_ms,
            queue_capacity,
            cost,
        };
        log(&format!(
            "[Rust/Wasm] Node added with spec: id={}, type={}, max_concurrent={}, process_time={}ms, queue={}, cost={}",
            id, node_type, max_concurrent, process_time_ms, queue_capacity, cost
        ));
        self.nodes.push(node);
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

        // 2. ノードでの処理時間を進める
        self.process_nodes(delta_ms);

        // 3. アクティブなパケットを更新
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

    /// 現在の経過時間を返す
    pub fn get_current_time(&self) -> f64 {
        self.current_time
    }

    /// 統計: 生成されたパケット総数
    pub fn get_stats_spawned(&self) -> u32 {
        self.stats.packets_spawned
    }

    /// 統計: 処理完了したパケット数
    pub fn get_stats_processed(&self) -> u32 {
        self.stats.packets_processed
    }

    /// 統計: ドロップしたパケット数
    pub fn get_stats_dropped(&self) -> u32 {
        self.stats.packets_dropped
    }

    /// 統計をリセット
    pub fn reset_stats(&mut self) {
        self.stats = SimulationStats::default();
        log("[Rust/Wasm] Stats reset");
    }

    /// シミュレーション全体をリセット（パケット、統計、時間）
    pub fn reset(&mut self) {
        // すべてのパケットを非アクティブに
        for packet in self.packets.iter_mut() {
            packet.active = 0;
        }
        // スポーンキューをクリア
        self.spawn_queue.clear();
        // 時間をリセット
        self.current_time = 0.0;
        // 統計をリセット
        self.stats = SimulationStats::default();
        log("[Rust/Wasm] Simulation reset");
    }

}

// SimulationStateの内部実装（#[wasm_bindgen]なし）- ノード位置取得
impl SimulationState {
    /// 指定IDのノード位置を取得（見つからない場合はNone）
    pub fn get_node_position(&self, id: u32) -> Option<(f32, f32)> {
        self.nodes.iter().find(|n| n.id == id).map(|n| (n.x, n.y))
    }

    /// インデックスでノード位置を取得
    pub fn get_node_position_by_index(&self, index: usize) -> Option<(f32, f32)> {
        self.nodes.get(index).map(|n| (n.x, n.y))
    }

    /// インデックスでノードタイプを取得
    pub fn get_node_type_by_index(&self, index: usize) -> Option<u32> {
        self.nodes.get(index).map(|n| n.node_type)
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
                self.stats.packets_spawned += actually_spawned as u32;
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

    /// アクティブなパケットの位置を更新（移動中のパケットのみ）
    fn update_packets(&mut self, _delta_ms: f64) {
        // 到達したパケットのインデックスを収集
        let mut arrived_packets: Vec<usize> = Vec::new();

        // まずパケットの移動処理（不変借用でノードを参照）
        for (idx, packet) in self.packets.iter_mut().enumerate() {
            if packet.active == 1 && packet.state == PacketState::Moving {
                // 移動中のパケットのみ処理
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

    /// パケットがターゲットノードに到達したときの処理（負荷モデル対応）
    fn handle_packet_arrival(&mut self, packet_idx: usize) {
        let target_node_idx = self.packets[packet_idx].target_node_idx;

        // ターゲットが存在しないなら終了
        if target_node_idx < 0 || (target_node_idx as usize) >= self.nodes.len() {
            self.packets[packet_idx].active = 0;
            return;
        }

        let node_idx = target_node_idx as usize;
        
        // ノードの情報を取得
        let node_type = self.nodes[node_idx].node_type;
        let process_time = self.nodes[node_idx].spec.process_time_ms;
        let max_concurrent = self.nodes[node_idx].spec.max_concurrent;
        let queue_capacity = self.nodes[node_idx].spec.queue_capacity;
        let current_processing = self.nodes[node_idx].processing_packets.len() as u32;
        let current_queue = self.nodes[node_idx].queue.len() as u32;
        let node_pos = (self.nodes[node_idx].x, self.nodes[node_idx].y);

        // パケット位置をノード位置に更新
        self.packets[packet_idx].x = node_pos.0;
        self.packets[packet_idx].y = node_pos.1;
        self.packets[packet_idx].current_node_idx = node_idx as i32;

        // 処理時間が0のノード（Gateway等）は即座に次へ転送
        if process_time <= 0.0 {
            self.route_packet_to_next(packet_idx, node_type, node_pos);
            return;
        }

        // 負荷チェック: 処理可能か？
        if current_processing < max_concurrent {
            // 処理開始
            self.packets[packet_idx].state = PacketState::Processing;
            self.nodes[node_idx].processing_packets.push(ProcessingPacket {
                packet_idx,
                remaining_time_ms: process_time,
            });
        } else if current_queue < queue_capacity {
            // キューに追加
            self.packets[packet_idx].state = PacketState::Queued;
            self.nodes[node_idx].queue.push(QueuedPacket { packet_idx });
        } else {
            // ドロップ！
            self.packets[packet_idx].active = 0;
            self.nodes[node_idx].total_dropped += 1;
            self.stats.packets_dropped += 1;
        }
    }

    /// パケットを次のノードへルーティング
    fn route_packet_to_next(&mut self, packet_idx: usize, current_node_type: u32, current_pos: (f32, f32)) {
        let next_node = match current_node_type {
            0 => self.find_next_node_by_type(1), // Gateway -> LB
            1 => self.find_next_server_target(), // LB -> Server (負荷分散)
            2 => self.find_next_node_by_type(3), // Server -> DB
            3 => {
                // DB到達 = 処理完了
                self.packets[packet_idx].active = 0;
                self.stats.packets_processed += 1;
                return;
            }
            _ => None,
        };

        if let Some(next_idx) = next_node {
            let p = &mut self.packets[packet_idx];
            p.target_node_idx = next_idx as i32;
            p.current_node_idx = -1; // 移動中
            p.state = PacketState::Moving;
            p.x = current_pos.0;
            p.y = current_pos.1;
        } else {
            // 次のノードがない = ドロップ
            self.packets[packet_idx].active = 0;
            self.stats.packets_dropped += 1;
        }
    }

    /// ノードでの処理時間を進め、完了したパケットを次へ送る
    fn process_nodes(&mut self, delta_ms: f64) {
        // 処理完了したパケットを収集
        let mut completed: Vec<(usize, usize)> = Vec::new(); // (node_idx, packet_idx)

        // 各ノードの処理時間を減算
        for (node_idx, node) in self.nodes.iter_mut().enumerate() {
            let mut completed_indices = Vec::new();
            
            for (i, proc) in node.processing_packets.iter_mut().enumerate() {
                proc.remaining_time_ms -= delta_ms;
                if proc.remaining_time_ms <= 0.0 {
                    completed_indices.push(i);
                    completed.push((node_idx, proc.packet_idx));
                }
            }

            // 処理完了したものを削除（逆順）
            for i in completed_indices.into_iter().rev() {
                node.processing_packets.remove(i);
                node.total_processed += 1;
            }

            // キューから次のパケットを処理開始
            while node.processing_packets.len() < node.spec.max_concurrent as usize
                && !node.queue.is_empty()
            {
                let queued = node.queue.remove(0);
                node.processing_packets.push(ProcessingPacket {
                    packet_idx: queued.packet_idx,
                    remaining_time_ms: node.spec.process_time_ms,
                });
                // パケットの状態を更新
                if queued.packet_idx < self.packets.len() {
                    self.packets[queued.packet_idx].state = PacketState::Processing;
                }
            }
        }

        // 処理完了したパケットを次のノードへルーティング
        for (node_idx, packet_idx) in completed {
            if packet_idx < self.packets.len() && self.packets[packet_idx].active == 1 {
                let node_type = self.nodes[node_idx].node_type;
                let node_pos = (self.nodes[node_idx].x, self.nodes[node_idx].y);
                self.route_packet_to_next(packet_idx, node_type, node_pos);
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

    /// ロードバランシング: 最も負荷の低いServerを選択
    fn find_next_server_target(&self) -> Option<usize> {
        // node_type == 2 (Server) のノードを収集
        let servers: Vec<(usize, f32)> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.node_type == 2)
            .map(|(i, node)| {
                // 負荷率 = (処理中 + キュー) / max_concurrent
                let load = (node.processing_packets.len() + node.queue.len()) as f32
                    / node.spec.max_concurrent.max(1) as f32;
                (i, load)
            })
            .collect();

        if servers.is_empty() {
            None
        } else {
            // 最も負荷の低いサーバーを選択
            servers
                .iter()
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| *idx)
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

    /// 各ノードの負荷率を取得（0.0 - 1.0+）
    /// 戻り値: [node0_load, node1_load, ...]
    pub fn get_node_load_rates(&self) -> Vec<f32> {
        self.nodes
            .iter()
            .map(|node| {
                if node.spec.max_concurrent == 0 {
                    0.0
                } else {
                    // 処理中 + キュー待ちの合計を考慮
                    let total_load = node.processing_packets.len() + node.queue.len();
                    total_load as f32 / node.spec.max_concurrent as f32
                }
            })
            .collect()
    }
}
