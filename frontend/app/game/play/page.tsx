'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useWasm } from '@/hooks/useWasm';
import { useWebSocket } from '@/hooks/useWebSocket';
import { PacketCanvas } from '@/components/PacketCanvas';
import { StatusIndicator } from '@/components/StatusIndicator';
import { StatsDisplay } from '@/components/StatsDisplay';
import { LogContainer } from '@/components/LogContainer';
import { Controls } from '@/components/Controls';
import { type NodeData, WASM_NODE_POSITIONS } from '@/components/NodeOverlay';

export default function Home() {
  const { isLoaded, isGpuReady, error, wasm, initGpu } = useWasm();
  const {
    isConnected,
    logs,
    packetCount,
    connect,
    sendTest,
    addLog,
  } = useWebSocket(wasm);

  const [isSimulationRunning, setIsSimulationRunning] = useState(false);
  const [activePacketCount, setActivePacketCount] = useState(0);
  const lastTimeRef = useRef<number>(0);
  const animationFrameRef = useRef<number | null>(null);

  // デバッグ用
  const [showDebugGrid, setShowDebugGrid] = useState(false);
  const [lbOffsetX, setLbOffsetX] = useState(0);
  const [lbOffsetY, setLbOffsetY] = useState(0);

  // ノード定義（Wasm座標 + デバッグオフセット）
  const nodes: NodeData[] = useMemo(() => [
    { id: 0, x: WASM_NODE_POSITIONS.gateway.x, y: WASM_NODE_POSITIONS.gateway.y, type: 'gateway', label: 'Gateway' },
    { id: 1, x: WASM_NODE_POSITIONS.lb.x + lbOffsetX, y: WASM_NODE_POSITIONS.lb.y + lbOffsetY, type: 'lb', label: 'LB' },
    { id: 2, x: WASM_NODE_POSITIONS.servers[0].x, y: WASM_NODE_POSITIONS.servers[0].y, type: 'server', label: 'Server 1' },
    { id: 3, x: WASM_NODE_POSITIONS.servers[1].x, y: WASM_NODE_POSITIONS.servers[1].y, type: 'server', label: 'Server 2' },
    { id: 4, x: WASM_NODE_POSITIONS.servers[2].x, y: WASM_NODE_POSITIONS.servers[2].y, type: 'server', label: 'Server 3' },
    { id: 5, x: WASM_NODE_POSITIONS.db.x, y: WASM_NODE_POSITIONS.db.y, type: 'db', label: 'DB' },
  ], [lbOffsetX, lbOffsetY]);

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
  // Canvas: 1920x1080, 中央: (960, 540)
  useEffect(() => {
    if (wasm && isGpuReady) {
      // ノードをクリアしてから配置
      wasm.simulation_clear_nodes();
      
      // ノードタイプ: 0=Gateway, 1=LB, 2=Server, 3=DB
      // Gateway（パケットの入口） - 画面左端
      wasm.simulation_add_node(0, 150, 540, 0);
      
      // LB（ロードバランサー） - 画面左寄り（初期位置）
      wasm.simulation_add_node(1, 550, 540, 1);
      
      // Servers（アプリサーバー） - 画面中央
      wasm.simulation_add_node(2, 1050, 270, 2);
      wasm.simulation_add_node(3, 1050, 540, 2);
      wasm.simulation_add_node(4, 1050, 810, 2);
      
      // DB（データベース） - 画面右寄り
      wasm.simulation_add_node(5, 1550, 540, 3);
      
      addLog('JS', `Nodes configured: ${wasm.simulation_get_node_count()} nodes`);
    }
  }, [wasm, isGpuReady, addLog]);

  // LB位置のスライダー変更時にWASM側のノード位置も更新
  useEffect(() => {
    if (wasm && isGpuReady) {
      const newX = WASM_NODE_POSITIONS.lb.x + lbOffsetX;
      const newY = WASM_NODE_POSITIONS.lb.y + lbOffsetY;
      wasm.simulation_update_node_position(1, newX, newY);
    }
  }, [wasm, isGpuReady, lbOffsetX, lbOffsetY]);

  // Test: spawn packets from center (random direction)
  const handleDebugSpawn = useCallback(() => {
    if (wasm && isGpuReady) {
      wasm.simulation_debug_spawn(960, 540, 100);
      setIsSimulationRunning(true);
      addLog('JS', 'debug_spawn: 100 packets from center');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: Auto-routing flow (Gateway → LB → Server → DB)
  // パケットはGatewayに到着後、自動的にLB→Server→DBとルーティングされる
  const handleAutoRoutingTest = useCallback(() => {
    if (wasm && isGpuReady) {
      // 左端からGateway（インデックス0）に向かってパケットを生成
      // Gateway到達後は自動でLB→Server→DBとルーティングされる
      wasm.simulation_spawn_wave_to_node(
        -20, 540,    // source position (off-screen left, vertically centered)
        0,           // target_node_idx (Gateway)
        50,          // count
        1000,        // duration_ms
        6.0,         // base_speed (increased for larger canvas)
        1.5,         // speed_variance
        0,           // packet_type (Normal)
        10           // complexity
      );
      setIsSimulationRunning(true);
      addLog('JS', 'Auto-routing test: 50 packets → Gateway (→ LB → Server → DB)');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn wave toward LB node
  const handleSpawnToLB = useCallback(() => {
    if (wasm && isGpuReady) {
      // 左端からLBノード（インデックス1）に向かってパケットを生成
      // LB到達後は自動でServer→DBとルーティングされる
      wasm.simulation_spawn_wave_to_node(
        -20, 540,    // source position (off-screen left, vertically centered)
        1,           // target_node_idx (LB)
        100,         // count
        1000,        // duration_ms
        6.0,         // base_speed (increased for larger canvas)
        1.5,         // speed_variance
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
      wasm.simulation_spawn_wave_to_node(550, 540, 2, 50, 500, 7.0, 2.0, 0, 10);
      // Server 2 (中央)
      wasm.simulation_spawn_wave_to_node(550, 540, 3, 50, 500, 7.0, 2.0, 0, 10);
      // Server 3 (下)
      wasm.simulation_spawn_wave_to_node(550, 540, 4, 50, 500, 7.0, 2.0, 0, 10);
      
      setIsSimulationRunning(true);
      addLog('JS', 'spawn_wave: 150 packets → Servers');
    }
  }, [wasm, isGpuReady, addLog]);

  // Test: spawn wave toward DB
  const handleSpawnToDB = useCallback(() => {
    if (wasm && isGpuReady) {
      // 各サーバーからDBにパケットを送信
      wasm.simulation_spawn_wave_to_node(1050, 270, 5, 30, 300, 6.0, 1.5, 0, 10);
      wasm.simulation_spawn_wave_to_node(1050, 540, 5, 30, 300, 6.0, 1.5, 0, 10);
      wasm.simulation_spawn_wave_to_node(1050, 810, 5, 30, 300, 6.0, 1.5, 0, 10);
      
      setIsSimulationRunning(true);
      addLog('JS', 'spawn_wave: 90 packets → DB');
    }
  }, [wasm, isGpuReady, addLog]);

  // Full flow demo: Gateway → LB → Servers → DB (Manual)
  const handleFullFlow = useCallback(() => {
    if (wasm && isGpuReady) {
      // Step 1: → LB
      wasm.simulation_spawn_wave_to_node(-20, 540, 1, 200, 2000, 6.0, 1.5, 0, 10);
      
      // Step 2: LB → Servers（少し遅延を持たせて）
      setTimeout(() => {
        if (wasm) {
          wasm.simulation_spawn_wave_to_node(550, 540, 2, 70, 1500, 7.0, 2.0, 0, 10);
          wasm.simulation_spawn_wave_to_node(550, 540, 3, 70, 1500, 7.0, 2.0, 0, 10);
          wasm.simulation_spawn_wave_to_node(550, 540, 4, 60, 1500, 7.0, 2.0, 0, 10);
        }
      }, 1000);
      
      // Step 3: Servers → DB
      setTimeout(() => {
        if (wasm) {
          wasm.simulation_spawn_wave_to_node(1050, 270, 5, 50, 1000, 6.0, 1.5, 0, 10);
          wasm.simulation_spawn_wave_to_node(1050, 540, 5, 50, 1000, 6.0, 1.5, 0, 10);
          wasm.simulation_spawn_wave_to_node(1050, 810, 5, 50, 1000, 6.0, 1.5, 0, 10);
        }
      }, 2200);
      
      setIsSimulationRunning(true);
      addLog('JS', 'Full flow: → LB → Servers → DB');
    }
  }, [wasm, isGpuReady, addLog]);

  const handleStopSimulation = useCallback(() => {
    setIsSimulationRunning(false);
    addLog('JS', 'Simulation stopped');
  }, [addLog]);

  return (
    <main className="min-h-screen p-4">
      <div className="w-full mx-auto space-y-4">
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
            nodes={nodes}
            showDebugGrid={showDebugGrid}
            lbOffsetX={lbOffsetX}
            lbOffsetY={lbOffsetY}
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

        {/* Debug: LB Offset Adjustment */}
        {isGpuReady && (
          <div className="px-4 py-3 bg-[#161b22] rounded-lg border border-[#30363d] space-y-3">
            <div className="flex items-center gap-4">
              <span className="text-[#f0883e] text-sm font-semibold">🔧 Debug</span>
              <button
                onClick={() => setShowDebugGrid(!showDebugGrid)}
                className={`px-3 py-1 rounded text-sm ${showDebugGrid ? 'bg-[#f0883e] text-white' : 'bg-[#30363d] text-[#8b949e] hover:bg-[#484f58]'}`}
              >
                {showDebugGrid ? '📐 Grid ON' : '📐 Grid OFF'}
              </button>
            </div>
            <div className="text-[#6e7681] text-xs">
              LBアイコンのオフセット調整（Wasm位置: {WASM_NODE_POSITIONS.lb.x}, {WASM_NODE_POSITIONS.lb.y}）
            </div>
            <div className="flex items-center gap-4">
              <label className="text-[#8b949e] text-sm w-32">X Offset: {lbOffsetX}</label>
              <input
                type="range"
                min="-500"
                max="500"
                value={lbOffsetX}
                onChange={(e) => setLbOffsetX(Number(e.target.value))}
                className="flex-1 h-2 bg-[#30363d] rounded-lg appearance-none cursor-pointer"
              />
            </div>
            <div className="flex items-center gap-4">
              <label className="text-[#8b949e] text-sm w-32">Y Offset: {lbOffsetY}</label>
              <input
                type="range"
                min="-500"
                max="500"
                value={lbOffsetY}
                onChange={(e) => setLbOffsetY(Number(e.target.value))}
                className="flex-1 h-2 bg-[#30363d] rounded-lg appearance-none cursor-pointer"
              />
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => { setLbOffsetX(0); setLbOffsetY(0); }}
                className="px-3 py-1 bg-[#30363d] hover:bg-[#484f58] rounded text-[#8b949e] text-sm"
              >
                Reset Offset (0, 0)
              </button>
              <span className="text-[#58a6ff] text-xs self-center">
                現在のLB位置: ({WASM_NODE_POSITIONS.lb.x + lbOffsetX}, {WASM_NODE_POSITIONS.lb.y + lbOffsetY})
              </span>
            </div>
          </div>
        )}

        {/* Simulation Controls */}
        {isGpuReady && (
          <div className="flex gap-2 flex-wrap">
            <button
              onClick={handleAutoRoutingTest}
              className="px-4 py-2 bg-[#da3633] hover:bg-[#f85149] rounded-lg text-white font-semibold transition-colors"
            >
              🚀 Auto Route
            </button>
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
              🔄 Full Flow (Manual)
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
