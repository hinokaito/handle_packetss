'use client';

import { useCallback } from 'react';
import { useWasm } from '@/hooks/useWasm';
import { useWebSocket } from '@/hooks/useWebSocket';
import { PacketCanvas } from '@/components/PacketCanvas';
import { StatusIndicator } from '@/components/StatusIndicator';
import { StatsDisplay } from '@/components/StatsDisplay';
import { LogContainer } from '@/components/LogContainer';
import { Controls } from '@/components/Controls';

export default function Home() {
  const { isLoaded, isGpuReady, error, wasm, initGpu } = useWasm();
  const {
    isConnected,
    logs,
    packetCount,
    connect,
    sendTest,
    clearLogs,
    addLog,
  } = useWebSocket(wasm);

  const handleClear = useCallback(() => {
    if (wasm) {
      wasm.clear_packet_buffer();
      addLog('JS', 'Canvas cleared (WebGPU rendering)');
    }
  }, [wasm, addLog]);

  return (
    <main className="min-h-screen p-8">
      <div className="max-w-[800px] mx-auto space-y-4">
        {/* Title */}
        <h1 className="text-[#58a6ff] text-3xl font-bold mb-6">
          🔌 WebSocket + Wasm Demo
        </h1>

        {/* Loading/Error State */}
        {!isLoaded && !error && (
          <div className="px-4 py-3 bg-[#21262d] rounded-lg border border-[#30363d] text-[#8b949e]">
            Loading Wasm module...
          </div>
        )}

        {error && (
          <div className="px-4 py-3 bg-[#f8514926] rounded-lg border border-[#f85149] text-[#f85149]">
            Error: {error}
          </div>
        )}

        {/* Status Indicator */}
        <StatusIndicator isConnected={isConnected} />

        {/* Canvas */}
        {isLoaded && (
          <PacketCanvas
            wasm={wasm}
            isGpuReady={isGpuReady}
            onGpuInit={initGpu}
            onLog={addLog}
          />
        )}

        {/* Stats */}
        <StatsDisplay packetCount={packetCount} />

        {/* Log Container */}
        <LogContainer logs={logs} />

        {/* Controls */}
        <Controls
          isConnected={isConnected}
          onConnect={connect}
          onSendTest={sendTest}
          onClear={handleClear}
        />
      </div>
    </main>
  );
}
