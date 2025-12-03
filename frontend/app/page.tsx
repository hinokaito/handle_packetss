'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
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

  const [isSimulationRunning, setIsSimulationRunning] = useState(false);
  const [activePacketCount, setActivePacketCount] = useState(0);
  const lastTimeRef = useRef<number>(0);
  const animationFrameRef = useRef<number | null>(null);

  // Simulation animation loop
  useEffect(() => {
    if (!isSimulationRunning || !wasm || !isGpuReady) return;

    const loop = (currentTime: number) => {
      const deltaMs = lastTimeRef.current ? currentTime - lastTimeRef.current : 16.67;
      lastTimeRef.current = currentTime;

      // Update simulation
      wasm.simulation_tick(deltaMs);
      
      // Render
      wasm.render_simulation_frame();
      
      // Update active count for display
      setActivePacketCount(wasm.simulation_get_active_count());

      animationFrameRef.current = requestAnimationFrame(loop);
    };

    lastTimeRef.current = 0;
    animationFrameRef.current = requestAnimationFrame(loop);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [isSimulationRunning, wasm, isGpuReady]);

  const handleClear = useCallback(() => {
    if (wasm) {
      wasm.clear_packet_buffer();
      addLog('JS', 'Canvas cleared (WebGPU rendering)');
    }
  }, [wasm, addLog]);

  // Test: spawn packets from center
  const handleDebugSpawn = useCallback(() => {
    if (wasm && isGpuReady) {
      wasm.simulation_debug_spawn(400, 300, 100);
      setIsSimulationRunning(true);
      addLog('JS', 'debug_spawn: 100 packets from (400, 300)');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn wave toward target
  const handleSpawnWave = useCallback(() => {
    if (wasm && isGpuReady) {
      // Spawn 500 packets from left side toward center over 2 seconds
      wasm.simulation_spawn_wave(
        -50, 300,    // source position (off-screen left)
        400, 300,    // target position (center)
        500,         // count
        2000,        // duration_ms
        5.0,         // base_speed
        1.5,         // speed_variance
        0,           // packet_type (Normal)
        10           // complexity
      );
      setIsSimulationRunning(true);
      addLog('JS', 'spawn_wave: 500 packets over 2s');
    }
  }, [wasm, isGpuReady, addLog]);

  const handleStopSimulation = useCallback(() => {
    setIsSimulationRunning(false);
    addLog('JS', 'Simulation stopped');
  }, [addLog]);

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

        {/* Simulation Stats */}
        {isGpuReady && (
          <div className="px-4 py-3 bg-[#21262d] rounded-lg border border-[#30363d]">
            <div className="text-[#8b949e] text-sm">
              Simulation: {isSimulationRunning ? '▶️ Running' : '⏸️ Stopped'} | 
              Active Packets: <span className="text-[#58a6ff] font-mono">{activePacketCount}</span>
            </div>
          </div>
        )}

        {/* Simulation Controls */}
        {isGpuReady && (
          <div className="flex gap-2 flex-wrap">
            <button
              onClick={handleDebugSpawn}
              className="px-4 py-2 bg-[#238636] hover:bg-[#2ea043] rounded-lg text-white font-semibold transition-colors"
            >
              🎯 Debug Spawn (100)
            </button>
            <button
              onClick={handleSpawnWave}
              className="px-4 py-2 bg-[#1f6feb] hover:bg-[#388bfd] rounded-lg text-white font-semibold transition-colors"
            >
              🌊 Spawn Wave (500)
            </button>
            {isSimulationRunning && (
              <button
                onClick={handleStopSimulation}
                className="px-4 py-2 bg-[#da3633] hover:bg-[#f85149] rounded-lg text-white font-semibold transition-colors"
              >
                ⏹️ Stop
              </button>
            )}
          </div>
        )}

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
