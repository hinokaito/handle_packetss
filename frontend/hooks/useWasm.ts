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
        };

        // Pre-allocate packet buffer
        wasmModule.allocate_packet_buffer(MAX_PACKETS);
        
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

