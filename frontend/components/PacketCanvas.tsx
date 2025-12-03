'use client';

import { useEffect, useRef, useCallback } from 'react';
import type { WasmModule } from '@/hooks/useWasm';

interface PacketCanvasProps {
  wasm: WasmModule | null;
  isGpuReady: boolean;
  onGpuInit: (canvasId: string) => Promise<void>;
  onLog?: (source: 'JS' | 'Rust' | 'WS', message: string) => void;
}

const CANVAS_ID = 'packetCanvas';
const CANVAS_WIDTH = 800;
const CANVAS_HEIGHT = 600;

export function PacketCanvas({ wasm, isGpuReady, onGpuInit, onLog }: PacketCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number | null>(null);
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

  // Animation loop
  useEffect(() => {
    if (!wasm || !isGpuReady) return;

    const animate = () => {
      wasm.render_frame();
      animationRef.current = requestAnimationFrame(animate);
    };

    animationRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [wasm, isGpuReady]);

  const handleClear = useCallback(() => {
    if (wasm) {
      wasm.clear_packet_buffer();
      onLog?.('JS', 'Canvas cleared');
    }
  }, [wasm, onLog]);

  return (
    <div className="rounded-lg border border-[#30363d] overflow-hidden bg-[#0d1117]">
      <canvas
        ref={canvasRef}
        id={CANVAS_ID}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        className="block w-full h-auto"
      />
    </div>
  );
}

