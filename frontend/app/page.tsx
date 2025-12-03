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

  // ノード配置（GPU初期化後に一度だけ）
  useEffect(() => {
    if (wasm && isGpuReady) {
      // ノードをクリアしてから配置
      wasm.simulation_clear_nodes();
      
      // ノードタイプ: 0=Gateway, 1=LB, 2=Server, 3=DB
      // Gateway（パケットの入口） - 画面左
      wasm.simulation_add_node(0, 50, 300, 0);
      
      // LB（ロードバランサー） - 画面中央左
      wasm.simulation_add_node(1, 250, 300, 1);
      
      // Servers（アプリサーバー） - 画面中央右
      wasm.simulation_add_node(2, 500, 150, 2);
      wasm.simulation_add_node(3, 500, 300, 2);
      wasm.simulation_add_node(4, 500, 450, 2);
      
      // DB（データベース） - 画面右
      wasm.simulation_add_node(5, 700, 300, 3);
      
      addLog('JS', `Nodes configured: ${wasm.simulation_get_node_count()} nodes`);
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn packets from center (random direction)
  const handleDebugSpawn = useCallback(() => {
    if (wasm && isGpuReady) {
      wasm.simulation_debug_spawn(400, 300, 100);
      setIsSimulationRunning(true);
      addLog('JS', 'debug_spawn: 100 packets from (400, 300)');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn wave toward LB node
  const handleSpawnToLB = useCallback(() => {
    if (wasm && isGpuReady) {
      // 左端からLBノード（インデックス1）に向かってパケットを生成
      wasm.simulation_spawn_wave_to_node(
        -20, 300,    // source position (off-screen left)
        1,           // target_node_idx (LB)
        100,         // count
        1000,        // duration_ms
        4.0,         // base_speed
        1.0,         // speed_variance
        0,           // packet_type (Normal)
        10           // complexity
      );
      setIsSimulationRunning(true);
      addLog('JS', 'spawn_wave: 100 packets → LB node');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn wave toward Server nodes
  const handleSpawnToServers = useCallback(() => {
    if (wasm && isGpuReady) {
      // LB位置から各サーバーにパケットを分散
      // Server 1 (上)
      wasm.simulation_spawn_wave_to_node(250, 300, 2, 50, 500, 5.0, 1.5, 0, 10);
      // Server 2 (中央)
      wasm.simulation_spawn_wave_to_node(250, 300, 3, 50, 500, 5.0, 1.5, 0, 10);
      // Server 3 (下)
      wasm.simulation_spawn_wave_to_node(250, 300, 4, 50, 500, 5.0, 1.5, 0, 10);
      
      setIsSimulationRunning(true);
      addLog('JS', 'spawn_wave: 150 packets → Servers');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn wave toward DB
  const handleSpawnToDB = useCallback(() => {
    if (wasm && isGpuReady) {
      // 各サーバーからDBにパケットを送信
      wasm.simulation_spawn_wave_to_node(500, 150, 5, 30, 300, 4.5, 1.0, 0, 10);
      wasm.simulation_spawn_wave_to_node(500, 300, 5, 30, 300, 4.5, 1.0, 0, 10);
      wasm.simulation_spawn_wave_to_node(500, 450, 5, 30, 300, 4.5, 1.0, 0, 10);
      
      setIsSimulationRunning(true);
      addLog('JS', 'spawn_wave: 90 packets → DB');
    }
  }, [wasm, isGpuReady, addLog]);

  // Full flow demo: Gateway → LB → Servers → DB
  const handleFullFlow = useCallback(() => {
    if (wasm && isGpuReady) {
      // Step 1: Gateway → LB
      wasm.simulation_spawn_wave_to_node(-20, 300, 1, 200, 2000, 4.0, 1.0, 0, 10);
      
      // Step 2: LB → Servers（少し遅延を持たせて）
      setTimeout(() => {
        if (wasm) {
          wasm.simulation_spawn_wave_to_node(250, 300, 2, 70, 1500, 5.0, 1.5, 0, 10);
          wasm.simulation_spawn_wave_to_node(250, 300, 3, 70, 1500, 5.0, 1.5, 0, 10);
          wasm.simulation_spawn_wave_to_node(250, 300, 4, 60, 1500, 5.0, 1.5, 0, 10);
        }
      }, 800);
      
      // Step 3: Servers → DB
      setTimeout(() => {
        if (wasm) {
          wasm.simulation_spawn_wave_to_node(500, 150, 5, 50, 1000, 4.5, 1.0, 0, 10);
          wasm.simulation_spawn_wave_to_node(500, 300, 5, 50, 1000, 4.5, 1.0, 0, 10);
          wasm.simulation_spawn_wave_to_node(500, 450, 5, 50, 1000, 4.5, 1.0, 0, 10);
        }
      }, 1800);
      
      setIsSimulationRunning(true);
      addLog('JS', 'Full flow: Gateway → LB → Servers → DB');
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
              onClick={handleSpawnToLB}
              className="px-4 py-2 bg-[#1f6feb] hover:bg-[#388bfd] rounded-lg text-white font-semibold transition-colors"
            >
              📡 → LB
            </button>
            <button
              onClick={handleSpawnToServers}
              className="px-4 py-2 bg-[#238636] hover:bg-[#2ea043] rounded-lg text-white font-semibold transition-colors"
            >
              🖥️ → Servers
            </button>
            <button
              onClick={handleSpawnToDB}
              className="px-4 py-2 bg-[#8957e5] hover:bg-[#a371f7] rounded-lg text-white font-semibold transition-colors"
            >
              💾 → DB
            </button>
            <button
              onClick={handleFullFlow}
              className="px-4 py-2 bg-[#f0883e] hover:bg-[#d29922] rounded-lg text-white font-semibold transition-colors"
            >
              🔄 Full Flow
            </button>
            <button
              onClick={handleDebugSpawn}
              className="px-4 py-2 bg-[#484f58] hover:bg-[#6e7681] rounded-lg text-white font-semibold transition-colors"
            >
              🎯 Random
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
