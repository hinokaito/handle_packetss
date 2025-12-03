'use client';

import { useEffect, useRef } from 'react';
import type { WasmModule } from '@/hooks/useWasm';

interface PacketCanvasProps {
  wasm: WasmModule | null;
  isGpuReady: boolean;
  onGpuInit: (canvasId: string) => Promise<void>;
  onLog?: (source: 'JS' | 'Rust' | 'WS', message: string) => void;
}

const CANVAS_ID = 'packetCanvas';
const CANVAS_WIDTH = 1920;
const CANVAS_HEIGHT = 1080;

export function PacketCanvas({ wasm, isGpuReady, onGpuInit, onLog }: PacketCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const initializedRef = useRef(false);

  // Initialize WebGPU when wasm is ready
  useEffect(() => {
    if (!wasm || initializedRef.current) return;

    const initializeGpu = async () => {
      try {
        onLog?.('JS', 'Initializing WebGPU...');
        await onGpuInit(CANVAS_ID);
        initializedRef.current = true;
        onLog?.('JS', 'WebGPU initialized successfully!');
      } catch (err) {
        onLog?.('JS', `WebGPU initialization failed: ${err}`);
        console.error('WebGPU init error:', err);
      }
    };

    initializeGpu();
  }, [wasm, onGpuInit, onLog]);

  // Note: Animation loop is handled by page.tsx using render_simulation_frame()

  // アスペクト比 16:9 を維持（1920:1080）
  return (
    <div className="flex justify-center">
      <div 
        className="rounded-lg border border-[#30363d] overflow-hidden bg-[#0d1117]"
        style={{ 
          width: 'min(70vw, 90vh * 16 / 9)',  // 幅は70vwか、高さベースの16:9のどちらか小さい方
          aspectRatio: '16 / 9'               // アスペクト比を固定
        }}
      >
        <canvas
          ref={canvasRef}
          id={CANVAS_ID}
          width={CANVAS_WIDTH}
          height={CANVAS_HEIGHT}
          className="block w-full h-full"
        />
      </div>
    </div>
  );
}

