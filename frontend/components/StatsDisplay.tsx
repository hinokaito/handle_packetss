'use client';

interface StatsDisplayProps {
  packetCount: number;
  jsonSize?: number;
  drawTime?: number;
}

export function StatsDisplay({ packetCount, jsonSize = 0, drawTime = 0 }: StatsDisplayProps) {
  return (
    <div className="flex gap-6 px-4 py-3 bg-[#21262d] rounded-lg border border-[#30363d] text-sm">
      <span className="text-[#8b949e]">
        Packets: <strong className="text-[#58a6ff] font-bold">{packetCount.toLocaleString()}</strong>
      </span>
      <span className="text-[#8b949e]">
        JSON Size: <strong className="text-[#58a6ff] font-bold">{(jsonSize / 1024).toFixed(2)} KB</strong>
      </span>
      <span className="text-[#8b949e]">
        Draw Time: <strong className="text-[#58a6ff] font-bold">{drawTime.toFixed(2)}ms</strong>
      </span>
    </div>
  );
}

