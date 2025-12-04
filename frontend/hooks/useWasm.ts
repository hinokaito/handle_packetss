'use client';

import { useState, useEffect, useCallback, useRef } from 'react';

// Wasm module types
export interface WasmModule {
  init_gpu: (canvasId: string) => Promise<boolean>;
  render_frame: () => void;
  handle_message: (message: string) => void;
  handle_binary: (data: Uint8Array) => void;
  allocate_packet_buffer: (capacity: number) => void;
  clear_packet_buffer: () => void;
  get_memory: () => WebAssembly.Memory;
  get_packet_buffer_ptr: () => number;
  get_packet_buffer_len: () => number;
  update_packet_buffer_from_binary: (data: Uint8Array) => number;
  update_packet_buffer_from_json: (jsonData: string) => number;
  console_log: (message: string) => void;
  // Simulation API
  create_simulation: (maxPackets: number) => void;
  simulation_spawn_wave: (
    x: number,
    y: number,
    targetX: number,
    targetY: number,
    count: number,
    durationMs: number,
    baseSpeed: number,
    speedVariance: number,
    packetType: number,
    complexity: number
  ) => void;
  simulation_spawn_wave_to_node: (
    x: number,
    y: number,
    targetNodeIdx: number,
    count: number,
    durationMs: number,
    baseSpeed: number,
    speedVariance: number,
    packetType: number,
    complexity: number
  ) => void;
  simulation_debug_spawn: (x: number, y: number, count: number) => void;
  simulation_tick: (deltaMs: number) => void;
  simulation_get_active_count: () => number;
  simulation_add_node: (id: number, x: number, y: number, nodeType: number) => void;
  simulation_clear_nodes: () => void;
  simulation_get_node_count: () => number;
  simulation_update_node_position: (id: number, x: number, y: number) => void;
  render_simulation_frame: () => void;
  // Stage Config API
  load_stage_config: (jsonStr: string) => boolean;
  get_stage_meta: () => string | undefined;
  get_stage_budget: () => number;
  get_stage_sla_target: () => number;
  trigger_waves_until: (currentTimeMs: number) => void;
  get_pending_wave_count: () => number;
  reset_stage_waves: () => void;
  // Stats API
  simulation_get_stats_spawned: () => number;
  simulation_get_stats_processed: () => number;
  simulation_get_stats_dropped: () => number;
  simulation_get_current_time: () => number;
  simulation_reset: () => void;
  simulation_get_node_position: (index: number) => Float32Array;
}

export interface UseWasmReturn {
  isLoaded: boolean;
  isGpuReady: boolean;
  error: string | null;
  wasm: WasmModule | null;
  initGpu: (canvasId: string) => Promise<void>;
}

const MAX_PACKETS = 100000;

export function useWasm(): UseWasmReturn {
  const [isLoaded, setIsLoaded] = useState(false);
  const [isGpuReady, setIsGpuReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wasmRef = useRef<WasmModule | null>(null);
  const initPromiseRef = useRef<Promise<void> | null>(null);

  // Load Wasm module on mount
  useEffect(() => {
    if (initPromiseRef.current) return;

    initPromiseRef.current = (async () => {
      try {
        // Dynamic import for client-side only
        const wasmModule = await import('@/lib/wasm/simulation');
        await wasmModule.default();

        wasmRef.current = {
          init_gpu: wasmModule.init_gpu,
          render_frame: wasmModule.render_frame,
          handle_message: wasmModule.handle_message,
          handle_binary: wasmModule.handle_binary,
          allocate_packet_buffer: wasmModule.allocate_packet_buffer,
          clear_packet_buffer: wasmModule.clear_packet_buffer,
          get_memory: wasmModule.get_memory,
          get_packet_buffer_ptr: wasmModule.get_packet_buffer_ptr,
          get_packet_buffer_len: wasmModule.get_packet_buffer_len,
          update_packet_buffer_from_binary: wasmModule.update_packet_buffer_from_binary,
          update_packet_buffer_from_json: wasmModule.update_packet_buffer_from_json,
          console_log: wasmModule.console_log,
          // Simulation API
          create_simulation: wasmModule.create_simulation,
          simulation_spawn_wave: wasmModule.simulation_spawn_wave,
          simulation_spawn_wave_to_node: wasmModule.simulation_spawn_wave_to_node,
          simulation_debug_spawn: wasmModule.simulation_debug_spawn,
          simulation_tick: wasmModule.simulation_tick,
          simulation_get_active_count: wasmModule.simulation_get_active_count,
          simulation_add_node: wasmModule.simulation_add_node,
          simulation_clear_nodes: wasmModule.simulation_clear_nodes,
          simulation_get_node_count: wasmModule.simulation_get_node_count,
          simulation_update_node_position: wasmModule.simulation_update_node_position,
          render_simulation_frame: wasmModule.render_simulation_frame,
          // Stage Config API
          load_stage_config: wasmModule.load_stage_config,
          get_stage_meta: wasmModule.get_stage_meta,
          get_stage_budget: wasmModule.get_stage_budget,
          get_stage_sla_target: wasmModule.get_stage_sla_target,
          trigger_waves_until: wasmModule.trigger_waves_until,
          get_pending_wave_count: wasmModule.get_pending_wave_count,
          reset_stage_waves: wasmModule.reset_stage_waves,
          // Stats API
          simulation_get_stats_spawned: wasmModule.simulation_get_stats_spawned,
          simulation_get_stats_processed: wasmModule.simulation_get_stats_processed,
          simulation_get_stats_dropped: wasmModule.simulation_get_stats_dropped,
          simulation_get_current_time: wasmModule.simulation_get_current_time,
          simulation_reset: wasmModule.simulation_reset,
          simulation_get_node_position: wasmModule.simulation_get_node_position,
        };

        // Pre-allocate packet buffer
        wasmModule.allocate_packet_buffer(MAX_PACKETS);
        
        // Initialize simulation
        wasmModule.create_simulation(MAX_PACKETS);
        
        setIsLoaded(true);
        console.log('[useWasm] Wasm module loaded successfully');
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Failed to load Wasm module';
        setError(message);
        console.error('[useWasm] Error loading Wasm:', err);
      }
    })();
  }, []);

  const initGpu = useCallback(async (canvasId: string) => {
    if (!wasmRef.current) {
      setError('Wasm module not loaded');
      return;
    }

    try {
      await wasmRef.current.init_gpu(canvasId);
      setIsGpuReady(true);
      console.log('[useWasm] WebGPU initialized successfully');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to initialize WebGPU';
      setError(message);
      console.error('[useWasm] WebGPU init error:', err);
    }
  }, []);

  return {
    isLoaded,
    isGpuReady,
    error,
    wasm: wasmRef.current,
    initGpu,
  };
}

