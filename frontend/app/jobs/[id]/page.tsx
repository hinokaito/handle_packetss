'use client';

import { useEffect, useCallback } from 'react';
import { useParams } from 'next/navigation';
import { useWasm, useStageManager } from '@/hooks';
import { PacketCanvas } from '@/components/PacketCanvas';

// =============================================================================
// Job Detail Page - /jobs/[id]
// ステージマネージャーのテストページ
// =============================================================================

export default function JobDetailPage() {
  const params = useParams();
  const jobId = params.id as string;

  const { isLoaded, isGpuReady, error: wasmError, wasm, initGpu } = useWasm();
  const {
    phase,
    stageConfig,
    stats,
    error: stageError,
    loadStage,
    startSimulation,
    pauseSimulation,
    resumeSimulation,
    resetStage,
    showResult,
  } = useStageManager(wasm);

  // GPU初期化後にステージをロード
  useEffect(() => {
    if (isGpuReady && jobId && phase === 'IDLE') {
      loadStage(jobId);
    }
  }, [isGpuReady, jobId, phase, loadStage]);

  // ログ追加用（PacketCanvasで使用）
  const addLog = useCallback((source: string, message: string) => {
    console.log(`[${source}] ${message}`);
  }, []);

  const error = wasmError || stageError;

  return (
    <main className="min-h-screen bg-[#0d1117] p-6">
      <div className="max-w-7xl mx-auto space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-[#c9d1d9]">
              Job: {jobId}
            </h1>
            {stageConfig && (
              <p className="text-[#8b949e] mt-1">
                {stageConfig.meta.title} - {stageConfig.meta.description}
              </p>
            )}
          </div>
          <PhaseIndicator phase={phase} />
        </div>

        {/* Error */}
        {error && (
          <div className="px-4 py-3 bg-[#f8514926] rounded-lg border border-[#f85149] text-[#f85149]">
            Error: {error}
          </div>
        )}

        {/* Loading State */}
        {!isLoaded && !error && (
          <div className="px-4 py-3 bg-[#21262d] rounded-lg border border-[#30363d] text-[#8b949e]">
            Loading Wasm module...
          </div>
        )}

        {/* Stage Info */}
        {stageConfig && (
          <div className="grid grid-cols-3 gap-4">
            <InfoCard
              title="Budget"
              value={`$${stageConfig.meta.budget}`}
              color="#238636"
            />
            <InfoCard
              title="SLA Target"
              value={`${(stageConfig.meta.sla_target * 100).toFixed(0)}%`}
              color="#1f6feb"
            />
            <InfoCard
              title="Waves"
              value={`${stageConfig.waves.length}`}
              color="#8957e5"
            />
          </div>
        )}

        {/* Canvas */}
        {isLoaded && (
          <div className="rounded-lg overflow-hidden border border-[#30363d]">
            <PacketCanvas
              wasm={wasm}
              isGpuReady={isGpuReady}
              onGpuInit={initGpu}
              onLog={addLog}
              nodes={[]} // 固定ノードはRust側で管理
              showDebugGrid={false}
              lbOffsetX={0}
              lbOffsetY={0}
            />
          </div>
        )}

        {/* Stats */}
        <div className="grid grid-cols-5 gap-4">
          <StatCard title="Spawned" value={stats.spawned} />
          <StatCard title="Processed" value={stats.processed} color="#238636" />
          <StatCard title="Dropped" value={stats.dropped} color="#f85149" />
          <StatCard title="In Flight" value={stats.inFlight} color="#1f6feb" />
          <StatCard
            title="SLA Rate"
            value={`${(stats.slaRate * 100).toFixed(1)}%`}
            color={stats.slaRate >= (stageConfig?.meta.sla_target ?? 0.99) ? '#238636' : '#f85149'}
          />
        </div>

        {/* Elapsed Time */}
        <div className="text-center text-[#8b949e]">
          Elapsed: {(stats.elapsedMs / 1000).toFixed(2)}s
        </div>

        {/* Controls */}
        <div className="flex justify-center gap-4">
          {phase === 'BUILD' && (
            <button
              onClick={startSimulation}
              className="px-6 py-3 bg-[#238636] hover:bg-[#2ea043] rounded-lg text-white font-semibold transition-colors"
            >
              ▶️ Start Simulation
            </button>
          )}

          {phase === 'RUNNING' && (
            <button
              onClick={pauseSimulation}
              className="px-6 py-3 bg-[#f0883e] hover:bg-[#d29922] rounded-lg text-white font-semibold transition-colors"
            >
              ⏸️ Pause
            </button>
          )}

          {phase === 'PAUSED' && (
            <>
              <button
                onClick={resumeSimulation}
                className="px-6 py-3 bg-[#238636] hover:bg-[#2ea043] rounded-lg text-white font-semibold transition-colors"
              >
                ▶️ Resume
              </button>
              <button
                onClick={resetStage}
                className="px-6 py-3 bg-[#21262d] hover:bg-[#30363d] rounded-lg text-[#c9d1d9] font-semibold transition-colors border border-[#30363d]"
              >
                🔄 Reset
              </button>
            </>
          )}

          {phase === 'COMPLETED' && (
            <>
              <button
                onClick={showResult}
                className="px-6 py-3 bg-[#1f6feb] hover:bg-[#388bfd] rounded-lg text-white font-semibold transition-colors"
              >
                📊 Show Result
              </button>
              <button
                onClick={resetStage}
                className="px-6 py-3 bg-[#21262d] hover:bg-[#30363d] rounded-lg text-[#c9d1d9] font-semibold transition-colors border border-[#30363d]"
              >
                🔄 Try Again
              </button>
            </>
          )}

          {phase === 'RESULT' && (
            <button
              onClick={resetStage}
              className="px-6 py-3 bg-[#238636] hover:bg-[#2ea043] rounded-lg text-white font-semibold transition-colors"
            >
              🔄 Try Again
            </button>
          )}
        </div>

        {/* Result Panel */}
        {phase === 'RESULT' && stageConfig && (
          <ResultPanel stats={stats} slaTarget={stageConfig.meta.sla_target} />
        )}
      </div>
    </main>
  );
}

// =============================================================================
// Sub Components
// =============================================================================

function PhaseIndicator({ phase }: { phase: string }) {
  const colors: Record<string, string> = {
    IDLE: 'bg-[#484f58]',
    LOADING: 'bg-[#f0883e]',
    BUILD: 'bg-[#1f6feb]',
    RUNNING: 'bg-[#238636]',
    PAUSED: 'bg-[#f0883e]',
    COMPLETED: 'bg-[#8957e5]',
    RESULT: 'bg-[#8957e5]',
  };

  return (
    <span className={`px-3 py-1 rounded-full text-white text-sm font-medium ${colors[phase] || 'bg-[#484f58]'}`}>
      {phase}
    </span>
  );
}

function InfoCard({ title, value, color }: { title: string; value: string; color: string }) {
  return (
    <div className="px-4 py-3 bg-[#21262d] rounded-lg border border-[#30363d]">
      <div className="text-[#8b949e] text-sm">{title}</div>
      <div className="text-xl font-bold" style={{ color }}>{value}</div>
    </div>
  );
}

function StatCard({ title, value, color = '#c9d1d9' }: { title: string; value: number | string; color?: string }) {
  return (
    <div className="px-4 py-3 bg-[#161b22] rounded-lg border border-[#30363d] text-center">
      <div className="text-[#8b949e] text-xs">{title}</div>
      <div className="text-lg font-mono font-bold" style={{ color }}>
        {typeof value === 'number' ? value.toLocaleString() : value}
      </div>
    </div>
  );
}

function ResultPanel({ stats, slaTarget }: { stats: { spawned: number; processed: number; dropped: number; slaRate: number; elapsedMs: number }; slaTarget: number }) {
  const passed = stats.slaRate >= slaTarget;

  return (
    <div className={`p-6 rounded-lg border-2 ${passed ? 'bg-[#23863620] border-[#238636]' : 'bg-[#f8514920] border-[#f85149]'}`}>
      <div className="text-center">
        <div className={`text-4xl font-bold mb-2 ${passed ? 'text-[#238636]' : 'text-[#f85149]'}`}>
          {passed ? '✅ PASSED' : '❌ FAILED'}
        </div>
        <div className="text-[#8b949e]">
          SLA: {(stats.slaRate * 100).toFixed(1)}% / Target: {(slaTarget * 100).toFixed(0)}%
        </div>
        <div className="mt-4 grid grid-cols-3 gap-4 text-sm">
          <div>
            <span className="text-[#8b949e]">Total Requests:</span>{' '}
            <span className="text-[#c9d1d9] font-mono">{stats.spawned}</span>
          </div>
          <div>
            <span className="text-[#8b949e]">Success:</span>{' '}
            <span className="text-[#238636] font-mono">{stats.processed}</span>
          </div>
          <div>
            <span className="text-[#8b949e]">Failed:</span>{' '}
            <span className="text-[#f85149] font-mono">{stats.dropped}</span>
          </div>
        </div>
        <div className="mt-2 text-[#8b949e] text-sm">
          Time: {(stats.elapsedMs / 1000).toFixed(2)}s
        </div>
      </div>
    </div>
  );
}

