'use client';

interface StatusIndicatorProps {
  isConnected: boolean;
}

export function StatusIndicator({ isConnected }: StatusIndicatorProps) {
  return (
    <div className="flex items-center gap-2 px-4 py-3 bg-[#21262d] rounded-lg border border-[#30363d]">
      <div
        className={`w-2.5 h-2.5 rounded-full transition-all duration-300 ${
          isConnected
            ? 'bg-[#3fb950] shadow-[0_0_8px_#3fb950]'
            : 'bg-[#f85149]'
        }`}
      />
      <span className="text-[#c9d1d9] text-sm font-medium">
        {isConnected ? 'Connected' : 'Disconnected'}
      </span>
    </div>
  );
}

