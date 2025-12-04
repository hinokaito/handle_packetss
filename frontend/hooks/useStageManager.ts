'use client';

import { useState, useCallback, useRef, useEffect } from 'react';
import type { WasmModule } from './useWasm';
import type { StageConfig } from '@/types/stage';

// =============================================================================
// Stage Manager Hook - ステージライフサイクル管理
// =============================================================================

/** ステージのフェーズ */
export type StagePhase = 
  | 'IDLE'      // 初期状態
  | 'LOADING'   // ステージJSON取得中
  | 'BUILD'     // ユーザーがノード配置中（tick停止）
  | 'RUNNING'   // シミュレーション実行中
  | 'PAUSED'    // 一時停止
  | 'COMPLETED' // 全Wave終了
  | 'RESULT';   // 結果表示中

/** シミュレーション統計 */
export interface SimulationStats {
  spawned: number;    // 生成されたパケット総数
  processed: number;  // 正常に処理完了したパケット数
  dropped: number;    // ドロップしたパケット数
  inFlight: number;   // 現在処理中のパケット数
  elapsedMs: number;  // 経過時間（ミリ秒）
  slaRate: number;    // SLA達成率 (processed / spawned)
}

/** useStageManagerの戻り値 */
export interface UseStageManagerReturn {
  /** 現在のフェーズ */
  phase: StagePhase;
  /** ロード済みステージ設定 */
  stageConfig: StageConfig | null;
  /** 現在の統計 */
  stats: SimulationStats;
  /** エラーメッセージ */
  error: string | null;
  
  /** ステージをロード */
  loadStage: (stageId: string) => Promise<boolean>;
  /** シミュレーション開始 */
  startSimulation: () => void;
  /** シミュレーション一時停止 */
  pauseSimulation: () => void;
  /** シミュレーション再開 */
  resumeSimulation: () => void;
  /** ステージをリセット（最初から） */
  resetStage: () => void;
  /** 結果表示へ移行 */
  showResult: () => void;
}

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

const initialStats: SimulationStats = {
  spawned: 0,
  processed: 0,
  dropped: 0,
  inFlight: 0,
  elapsedMs: 0,
  slaRate: 0,
};

export function useStageManager(wasm: WasmModule | null): UseStageManagerReturn {
  const [phase, setPhase] = useState<StagePhase>('IDLE');
  const [stageConfig, setStageConfig] = useState<StageConfig | null>(null);
  const [stats, setStats] = useState<SimulationStats>(initialStats);
  const [error, setError] = useState<string | null>(null);

  const lastTimeRef = useRef<number>(0);
  const animationFrameRef = useRef<number | null>(null);

  /**
   * 統計を更新
   */
  const updateStats = useCallback(() => {
    if (!wasm) return;

    const spawned = wasm.simulation_get_stats_spawned();
    const processed = wasm.simulation_get_stats_processed();
    const dropped = wasm.simulation_get_stats_dropped();
    const inFlight = wasm.simulation_get_active_count();
    const elapsedMs = wasm.simulation_get_current_time();
    const slaRate = spawned > 0 ? processed / spawned : 0;

    setStats({
      spawned,
      processed,
      dropped,
      inFlight,
      elapsedMs,
      slaRate,
    });
  }, [wasm]);

  /**
   * シミュレーションループ
   */
  const simulationLoop = useCallback((currentTime: number) => {
    if (!wasm) return;

    const deltaMs = lastTimeRef.current ? currentTime - lastTimeRef.current : 16.67;
    lastTimeRef.current = currentTime;

    // Rustのcurrent_timeを取得してWaveを発火
    const simTime = wasm.simulation_get_current_time();
    wasm.trigger_waves_until(Math.floor(simTime));

    // シミュレーションを1フレーム進める
    wasm.simulation_tick(deltaMs);

    // 描画
    wasm.render_simulation_frame();

    // 統計を更新
    updateStats();

    // 全Wave終了 && アクティブパケット0 → COMPLETED
    const pendingWaves = wasm.get_pending_wave_count();
    const activeCount = wasm.simulation_get_active_count();
    if (pendingWaves === 0 && activeCount === 0 && stats.spawned > 0) {
      setPhase('COMPLETED');
      console.log('[useStageManager] Simulation completed');
      return; // ループ終了
    }

    animationFrameRef.current = requestAnimationFrame(simulationLoop);
  }, [wasm, updateStats, stats.spawned]);

  /**
   * ステージをロード
   */
  const loadStage = useCallback(async (stageId: string): Promise<boolean> => {
    if (!wasm) {
      setError('Wasm module not loaded');
      return false;
    }

    setPhase('LOADING');
    setError(null);

    try {
      // GoサーバーからJSONを取得
      const response = await fetch(`${API_BASE_URL}/api/stages/${stageId}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch stage: ${response.status}`);
      }

      const config: StageConfig = await response.json();
      setStageConfig(config);

      // RustにJSONをロード
      const jsonStr = JSON.stringify(config);
      const success = wasm.load_stage_config(jsonStr);
      
      if (!success) {
        throw new Error('Failed to load stage config in Rust');
      }

      // シミュレーションをリセット
      wasm.simulation_reset();
      setStats(initialStats);

      setPhase('BUILD');
      console.log('[useStageManager] Stage loaded:', config.meta.title);
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setError(message);
      setPhase('IDLE');
      console.error('[useStageManager] Error loading stage:', err);
      return false;
    }
  }, [wasm]);

  /**
   * シミュレーション開始
   */
  const startSimulation = useCallback(() => {
    if (!wasm || phase !== 'BUILD') return;

    // Waveをリセット（念のため）
    wasm.reset_stage_waves();
    
    // 統計をリセット
    wasm.simulation_reset();
    setStats(initialStats);

    setPhase('RUNNING');
    lastTimeRef.current = 0;
    animationFrameRef.current = requestAnimationFrame(simulationLoop);
    console.log('[useStageManager] Simulation started');
  }, [wasm, phase, simulationLoop]);

  /**
   * シミュレーション一時停止
   */
  const pauseSimulation = useCallback(() => {
    if (phase !== 'RUNNING') return;

    if (animationFrameRef.current) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }

    setPhase('PAUSED');
    console.log('[useStageManager] Simulation paused');
  }, [phase]);

  /**
   * シミュレーション再開
   */
  const resumeSimulation = useCallback(() => {
    if (phase !== 'PAUSED') return;

    setPhase('RUNNING');
    lastTimeRef.current = 0;
    animationFrameRef.current = requestAnimationFrame(simulationLoop);
    console.log('[useStageManager] Simulation resumed');
  }, [phase, simulationLoop]);

  /**
   * ステージをリセット
   */
  const resetStage = useCallback(() => {
    if (!wasm) return;

    // アニメーションを停止
    if (animationFrameRef.current) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }

    // シミュレーションをリセット
    wasm.simulation_reset();
    
    // Waveをリセット
    wasm.reset_stage_waves();
    
    // 統計をリセット
    setStats(initialStats);

    setPhase('BUILD');
    console.log('[useStageManager] Stage reset');
  }, [wasm]);

  /**
   * 結果表示へ移行
   */
  const showResult = useCallback(() => {
    if (phase !== 'COMPLETED') return;
    setPhase('RESULT');
    console.log('[useStageManager] Showing result');
  }, [phase]);

  // クリーンアップ
  useEffect(() => {
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  return {
    phase,
    stageConfig,
    stats,
    error,
    loadStage,
    startSimulation,
    pauseSimulation,
    resumeSimulation,
    resetStage,
    showResult,
  };
}

