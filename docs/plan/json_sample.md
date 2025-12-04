# Goから受け取るJSON(仮)
```json
{
    "meta": {
      "title": "Stage 1: First Access",
      "description": "予算$500以内で、秒間100リクエストを捌け。SLA 99%必達。",
      "budget": 500,
      "sla_target": 0.99
    },
    "map": {
      // ユーザーが配置するのではなく、初期配置として固定されているもの（Start地点など）
      "fixed_nodes": [
        { "id": "gateway", "type": "gateway", "x": 0, "y": 300 }
      ]
    },
    "waves": [
      // 敵の出現パターン
      {
        "time_start_ms": 1000,
        "source_id": "gateway",
        "count": 50,
        "duration_ms": 2000,
        "packet_type": "NORMAL",
        "speed": 5.0
      },
      {
        "time_start_ms": 5000,
        "source_id": "gateway",
        "count": 200, // ちょっと増える
        "duration_ms": 1000,
        "packet_type": "NORMAL",
        "speed": 6.0
      }
    ]
  }
```

# ユーザーの流れ

Stage Select: ユーザーが選択。
↓
Fetch & Load: GoからJSON取得 → TS経由でRustに「固定ノード（Start地点など）」と「Wave情報」をロード。
↓
Build Phase:
まだ時間は動きません（Tick停止中）。
ユーザーは与えられた予算内で、LBやサーバーを配置・配線します。
「よし、これで耐えられるはず！」と設計図を完成させる。
↓
Simulation Start:
ユーザーが「Start」ボタンを押す。
↓
Rustの tick が回り始める。
↓
設定された時間(time_start_ms)になると、Rust内部のスケジューラーが自動でパケットを吐き出す。
↓
Result: 全Wave終了後、スコア（SLA/コスト）が表示される。