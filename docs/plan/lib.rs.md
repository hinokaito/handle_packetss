# データ構造の定義
Rustの線形メモリに並べるデータ。
```rust
use wasm_bindgen::prelude::*;

// packet struct
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Packet {
    pub x: f32,
    pub y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub active: u32,      // 0: inactive, 1: active
    pub packet_type: u32, // 0: Normal, 1: Malicious
}


// simulation state struct
#[wasm_bindgen]
pub struct SimulationState {
    packets: Vec<Packet>, // list of packets
    max_packets: usize,   // maximum number of packets
}

```

# 公開メソッド(API)
JS側が呼び出す関数。Goは一旦放置して、ダミー生成関数もここにつくる
```rust
#[wasm_bindgen]
impl SimulationState {
    // コンストラクタ
    pub fn new(max_packets: usize) -> SimulationState {
        // 全パケットを初期化（active=0で埋める）
        let packets = vec![
            Packet { x: 0.0, y: 0.0, velocity_x: 0.0, velocity_y: 0.0, active: 0, packet_type: 0 };
            max_packets
        ];
        SimulationState { packets, max_packets }
    }

    // 毎フレーム呼ばれる関数 (The Loop)
    pub fn tick(&mut self) {
        for packet in self.packets.iter_mut() {
            if packet.active == 1 {
                // ここに移動ロジックを書く
                packet.x += packet.velocity_x;
                packet.y += packet.velocity_y;

                // 画面外に出たら消すなどの簡易ロジック
                if packet.x > 1000.0 || packet.y > 1000.0 {
                    packet.active = 0;
                }
            }
        }
    }

    // ★重要: 今はGoがいないので、テスト用に手動でパケットを湧かせる関数
    pub fn debug_spawn(&mut self, x: f32, y: f32, count: usize) {
        let mut spawned = 0;
        for packet in self.packets.iter_mut() {
            if packet.active == 0 {
                packet.active = 1;
                packet.x = x;
                packet.y = y;
                // 適当に散らばらせる
                packet.velocity_x = (js_sys::Math::random() as f32 - 0.5) * 2.0; 
                packet.velocity_y = (js_sys::Math::random() as f32 - 0.5) * 2.0;
                
                spawned += 1;
                if spawned >= count { break; }
            }
        }
    }

    // WebGPUに渡すためのメモリアドレス（ポインタ）を返す
    pub fn get_packets_ptr(&self) -> *const Packet {
        self.packets.as_ptr()
    }
}
```

TypeScript側は、起動時に debug_spawn(500, 500, 100) のように呼び出して、画面中央からパケットが噴き出すかだけを確認できるのが目標です


# ---更新!---
Goから流す情報です。

## 放出制御
- count(u32): 生成する総数
- duration_ms(u32): 何ミリかけて放出するか。(0なら即時、5000なら5秒かけて等間隔に、など。)
- wave_id(String/u64): この攻撃/トラフィックの識別子。集計時に「Wave1の攻撃を防げたか？」など追跡するため。

## 空間・物理
- ~~source_id(String)~~: 最終的に"Internet-A"のようなIDにしたいですが、今は次の行の通りに
- x(f32): 放出位置のX座標
- y(f32): 放出位置のY座標 ただしこれらは今後上のsouce_idに移行する可能性が高いです。
- target_id(String): 最初の目的地(例: "load-balancer-01") RustはこのIDをもつノードの座標を検索し、そこに向かうベクトルを計算します。
- base_speed(f32): パケットの基本移動速度。
- speed_variance(f32): パケットの速度のばらつき。

## パケットの質・属性
- packet_type(Enum): {
    Normal,    // 通常アクセス(利益)。
    SYN_FLOOD, // 処理負荷は低いが数が膨大で、接続数を埋めに来る。
    HEAVY_SQL, // 数は少ないが、処理時間(CPU負荷)が極大。
    KILLER,    // 触れたノードを確率でダウンさせる(カオスエンジニアリング用)。
  }
- complexity(u8): 「重さ」係数。サーバーノードに到達した際、この値が高いほど処理時間(待機時間)が長くなる。

## 見た目の強制指定
基本はpacket_typeで色を決めますが、イベント演出用に作っておく。
- color_override(Option<u32>): RGBA hex

## 具体的なJSONイメージ
```json
{
  "type": "SPAWN",
  "payload": {
    "wave_id": "wave_level1_05",
    
    // 1. Emission
    "count": 500,
    "duration_ms": 2000,  // 2秒かけて500個出す
    
    // 2. Spatial
    "source": { "x": -100.0, "y": 300.0 }, // 画面左外
    "target_id": "lb-primary",             // LBに向かえ
    "base_speed": 5.0,
    "speed_variance": 1.5,                 // 速度は 3.5 ~ 6.5 の間でランダム
    
    // 3. Logic
    "packet_type": "HEAVY_SQL",            // 重いリクエスト
    "complexity": 80,                      // サーバーを長時間占有する
    "payload_size": 1024                   // 帯域幅を食う(オプション)
  }
}
```

## Rust側での実装例
```rust
// lib.rs

// 外部(JS/Go)から指定されるパケットのスペック
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum PacketType {
    Normal = 0,
    SynFlood = 1,
    HeavyTask = 2,
    Killer = 3,
}

#[wasm_bindgen]
impl SimulationState {
    // 以前の debug_spawn を強化した本番用メソッド
    pub fn spawn_wave(
        &mut self,
        x: f32, 
        y: f32, 
        target_x: f32, // target_idからJS側で座標解決して渡すのが一番速い
        target_y: f32,
        count: usize,
        duration_ms: u32,
        base_speed: f32,
        speed_variance: f32,
        packet_type: PacketType,
        complexity: u8
    ) {
        // ここで「予約リスト」にタスクを追加する
        // 実際の生成(active=1にする処理)は tick() の中で
        // duration_ms に基づいて少しずつ行う
        self.spawn_queue.push(SpawnTask {
            x, y, target_x, target_y, count, duration_ms, 
            base_speed, speed_variance, packet_type, complexity,
            progress: 0, // 生成済みカウンタ
        });
    }
}
```