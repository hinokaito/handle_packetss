[English](README.md)

# handle_packetss

**インフラストラクチャーをゲームのように**\
Rust (Wasm)、WebGL、Go (QUIC) を使用したリアルタイムパケットトラフィックシミュレーション。

このプロジェクトは、100,000以上のネットワークパケットの流れを可視化し、ブラウザベースの環境でロードバランシングアルゴリズムをシミュレートします。


## コンセプト
- **大規模スケール:** WebGLインスタンシングを使用した高スループットトラフィックの可視化。
- **低レベル:** QUIC (WebTransport) 上のカスタムバイナリプロトコル。
- **シミュレーション:** GoバックエンドとWasmフロントエンド間のリアルタイムフィードバックループ。

## 技術スタック
- **フロントエンド:** Rust (Wasm), wgpu (WebGPU/WebGL), TypeScript
- **バックエンド:** Go, QUIC-go
- **プロトコル:** WebTransport

## 起動方法

### バックエンド
```bash
cd server
go run main.go
```

### フロントエンド
```bash
cd web
python -m http.server 8080
```

## ライセンス
**MIT License.**
これはポートフォリオプロジェクトです。

