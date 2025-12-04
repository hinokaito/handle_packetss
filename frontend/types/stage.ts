// =============================================================================
// Stage Types - GoサーバーのJSON形式と一致
// =============================================================================

/** ステージのメタ情報 */
export interface StageMeta {
  title: string;
  description: string;
  budget: number;
  sla_target: number;
}

/** 固定配置されるノード（Gateway等） */
export interface FixedNode {
  id: string;
  type: 'gateway' | 'lb' | 'server' | 'db';
  x: number;
  y: number;
}

/** マップ設定 */
export interface MapConfig {
  fixed_nodes: FixedNode[];
}

/** パケット出現パターン（Wave） */
export interface WaveConfig {
  time_start_ms: number;
  source_id: string;
  count: number;
  duration_ms: number;
  packet_type: 'NORMAL' | 'SYN_FLOOD' | 'HEAVY_TASK' | 'KILLER';
  speed: number;
}

/** ステージ全体の設定 */
export interface StageConfig {
  meta: StageMeta;
  map: MapConfig;
  waves: WaveConfig[];
}

/** ステージ一覧用の簡易情報 */
export interface StageListItem {
  id: string;
  title: string;
  description: string;
}

