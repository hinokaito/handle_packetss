'use client';

import { useEffect, useRef } from 'react';

// ノードの定義
export interface NodeData {
  id: number;
  x: number;
  y: number;
  type: 'gateway' | 'lb' | 'server' | 'db';
  label: string;
}

// Wasm側のノード座標（シミュレーション用）
export const WASM_NODE_POSITIONS = {
  gateway: { x: 150, y: 540 },
  lb: { x: 550, y: 540 },
  servers: [
    { x: 1050, y: 270 },
    { x: 1050, y: 540 },
    { x: 1050, y: 810 },
  ],
  db: { x: 1550, y: 540 },
};

interface NodeOverlayProps {
  nodes: NodeData[];
  width: number;
  height: number;
  showDebugGrid?: boolean;
  lbOffsetX?: number;
  lbOffsetY?: number;
}

// ノードタイプ別の色設定
const NODE_COLORS: Record<NodeData['type'], { fill: string; stroke: string; text: string }> = {
  gateway: { fill: '#238636', stroke: '#2ea043', text: '#ffffff' },  // 緑
  lb: { fill: '#1f6feb', stroke: '#388bfd', text: '#ffffff' },       // 青
  server: { fill: '#8957e5', stroke: '#a371f7', text: '#ffffff' },   // 紫
  db: { fill: '#f0883e', stroke: '#d29922', text: '#ffffff' },       // オレンジ
};

// ノードタイプ別のサイズ
const NODE_SIZES: Record<NodeData['type'], { width: number; height: number; radius: number }> = {
  gateway: { width: 80, height: 50, radius: 8 },
  lb: { width: 90, height: 50, radius: 25 },      // 楕円形
  server: { width: 70, height: 70, radius: 8 },   // 四角形
  db: { width: 60, height: 70, radius: 8 },       // 円筒形
};

export function NodeOverlay({ nodes, width, height, showDebugGrid = false, lbOffsetX = 0, lbOffsetY = 0 }: NodeOverlayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // クリア
    ctx.clearRect(0, 0, width, height);

    // デバッググリッド描画
    if (showDebugGrid) {
      ctx.strokeStyle = 'rgba(255, 255, 0, 0.3)';
      ctx.lineWidth = 1;
      
      // 縦線 (X方向のグリッド)
      for (let x = 0; x <= width; x += 200) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
        
        // 座標ラベル
        ctx.fillStyle = 'rgba(255, 255, 0, 0.7)';
        ctx.font = '12px monospace';
        ctx.fillText(`${x}`, x + 2, 15);
      }
      
      // 横線 (Y方向のグリッド)
      for (let y = 0; y <= height; y += 200) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
        
        ctx.fillStyle = 'rgba(255, 255, 0, 0.7)';
        ctx.fillText(`${y}`, 2, y + 15);
      }
      
      // 中央マーカー (960, 540)
      ctx.strokeStyle = 'rgba(255, 0, 0, 0.8)';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(width / 2 - 30, height / 2);
      ctx.lineTo(width / 2 + 30, height / 2);
      ctx.moveTo(width / 2, height / 2 - 30);
      ctx.lineTo(width / 2, height / 2 + 30);
      ctx.stroke();
      
      ctx.fillStyle = 'rgba(255, 0, 0, 0.8)';
      ctx.fillText('CENTER (960,540)', width / 2 + 5, height / 2 - 10);
      
      // Wasm実際の位置マーカー（シミュレーション上の座標）
      ctx.strokeStyle = 'rgba(255, 0, 255, 0.8)';
      ctx.lineWidth = 2;
      ctx.setLineDash([5, 5]);
      
      // Gateway
      ctx.beginPath();
      ctx.arc(WASM_NODE_POSITIONS.gateway.x, WASM_NODE_POSITIONS.gateway.y, 25, 0, Math.PI * 2);
      ctx.stroke();
      
      // LB（スライダーで動かした位置を表示）
      ctx.beginPath();
      ctx.arc(WASM_NODE_POSITIONS.lb.x + lbOffsetX, WASM_NODE_POSITIONS.lb.y + lbOffsetY, 25, 0, Math.PI * 2);
      ctx.stroke();
      
      // Servers
      WASM_NODE_POSITIONS.servers.forEach((s) => {
        ctx.beginPath();
        ctx.arc(s.x, s.y, 25, 0, Math.PI * 2);
        ctx.stroke();
      });
      
      // DB
      ctx.beginPath();
      ctx.arc(WASM_NODE_POSITIONS.db.x, WASM_NODE_POSITIONS.db.y, 25, 0, Math.PI * 2);
      ctx.stroke();
      
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(255, 0, 255, 0.8)';
      ctx.font = '10px monospace';
      ctx.fillText('● = Wasm位置（パケットの実際の目標）', 10, height - 10);
    }

    // 各ノードを描画
    nodes.forEach((node) => {
      const colors = NODE_COLORS[node.type];
      const size = NODE_SIZES[node.type];

      ctx.save();

      // ノードタイプ別の描画
      switch (node.type) {
        case 'gateway':
          // 矢印付き四角形
          drawRoundedRect(ctx, node.x - size.width / 2, node.y - size.height / 2, size.width, size.height, size.radius, colors);
          drawArrowRight(ctx, node.x + size.width / 2 - 5, node.y, 15, colors.stroke);
          break;

        case 'lb':
          // 楕円形（ロードバランサー）
          drawEllipse(ctx, node.x, node.y, size.width / 2, size.height / 2, colors);
          // デバッグ: LBの中心に赤い十字を描画
          ctx.strokeStyle = 'red';
          ctx.lineWidth = 3;
          ctx.beginPath();
          ctx.moveTo(node.x - 20, node.y);
          ctx.lineTo(node.x + 20, node.y);
          ctx.moveTo(node.x, node.y - 20);
          ctx.lineTo(node.x, node.y + 20);
          ctx.stroke();
          // 座標を表示
          ctx.fillStyle = 'red';
          ctx.font = 'bold 14px monospace';
          ctx.fillText(`(${node.x}, ${node.y})`, node.x + 25, node.y - 25);
          break;

        case 'server':
          // 四角形（サーバー）
          drawRoundedRect(ctx, node.x - size.width / 2, node.y - size.height / 2, size.width, size.height, size.radius, colors);
          // サーバーアイコン
          drawServerIcon(ctx, node.x, node.y - 10, colors.text);
          break;

        case 'db':
          // 円筒形（データベース）
          drawCylinder(ctx, node.x, node.y, size.width / 2, size.height / 2, colors);
          break;
      }

      // ラベル描画
      ctx.fillStyle = colors.text;
      ctx.font = 'bold 12px Inter, system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      
      const labelY = node.type === 'db' ? node.y + 5 : node.y + (node.type === 'server' ? 15 : 0);
      ctx.fillText(node.label, node.x, labelY);

      ctx.restore();
    });
  }, [nodes, width, height, showDebugGrid, lbOffsetX, lbOffsetY]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className="absolute inset-0 w-full h-full pointer-events-none"
    />
  );
}

// 角丸四角形を描画
function drawRoundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
  colors: { fill: string; stroke: string }
) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
  ctx.fillStyle = colors.fill;
  ctx.fill();
  ctx.strokeStyle = colors.stroke;
  ctx.lineWidth = 2;
  ctx.stroke();
}

// 楕円を描画
function drawEllipse(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  rx: number,
  ry: number,
  colors: { fill: string; stroke: string }
) {
  ctx.beginPath();
  ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
  ctx.fillStyle = colors.fill;
  ctx.fill();
  ctx.strokeStyle = colors.stroke;
  ctx.lineWidth = 2;
  ctx.stroke();
}

// 円筒形（DB）を描画
function drawCylinder(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  rx: number,
  ry: number,
  colors: { fill: string; stroke: string }
) {
  const ellipseHeight = ry * 0.3;
  const bodyHeight = ry * 2 - ellipseHeight * 2;

  // 本体（四角形）
  ctx.fillStyle = colors.fill;
  ctx.fillRect(cx - rx, cy - ry + ellipseHeight, rx * 2, bodyHeight);

  // 左右の線
  ctx.strokeStyle = colors.stroke;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(cx - rx, cy - ry + ellipseHeight);
  ctx.lineTo(cx - rx, cy + ry - ellipseHeight);
  ctx.moveTo(cx + rx, cy - ry + ellipseHeight);
  ctx.lineTo(cx + rx, cy + ry - ellipseHeight);
  ctx.stroke();

  // 下の楕円
  ctx.beginPath();
  ctx.ellipse(cx, cy + ry - ellipseHeight, rx, ellipseHeight, 0, 0, Math.PI * 2);
  ctx.fillStyle = colors.fill;
  ctx.fill();
  ctx.strokeStyle = colors.stroke;
  ctx.stroke();

  // 上の楕円
  ctx.beginPath();
  ctx.ellipse(cx, cy - ry + ellipseHeight, rx, ellipseHeight, 0, 0, Math.PI * 2);
  ctx.fillStyle = colors.stroke; // 少し明るく
  ctx.fill();
  ctx.strokeStyle = colors.stroke;
  ctx.stroke();
}

// 右向き矢印を描画
function drawArrowRight(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  size: number,
  color: string
) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.moveTo(x, y - size / 3);
  ctx.lineTo(x + size / 2, y);
  ctx.lineTo(x, y + size / 3);
  ctx.closePath();
  ctx.fill();
}

// サーバーアイコンを描画
function drawServerIcon(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  color: string
) {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  
  // 3本の水平線
  for (let i = -1; i <= 1; i++) {
    ctx.beginPath();
    ctx.moveTo(cx - 15, cy + i * 8);
    ctx.lineTo(cx + 15, cy + i * 8);
    ctx.stroke();
  }
}

