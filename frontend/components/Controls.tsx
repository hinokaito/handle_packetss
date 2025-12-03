'use client';

interface ControlsProps {
  isConnected: boolean;
  onConnect: () => void;
  onSendTest: () => void;
  onClear: () => void;
}

export function Controls({ isConnected, onConnect, onSendTest, onClear }: ControlsProps) {
  return (
    <div className="flex gap-2">
      <button
        onClick={onConnect}
        className={`px-4 py-2 rounded-lg font-medium text-sm transition-colors ${
          isConnected
            ? 'bg-[#f85149] hover:bg-[#da3633] text-white'
            : 'bg-[#238636] hover:bg-[#2ea043] text-white'
        }`}
      >
        {isConnected ? 'Disconnect' : 'Connect'}
      </button>
      <button
        onClick={onSendTest}
        disabled={!isConnected}
        className="px-4 py-2 rounded-lg font-medium text-sm bg-[#238636] hover:bg-[#2ea043] text-white transition-colors disabled:bg-[#21262d] disabled:text-[#8b949e] disabled:cursor-not-allowed"
      >
        Send Test
      </button>
      <button
        onClick={onClear}
        className="px-4 py-2 rounded-lg font-medium text-sm bg-[#21262d] hover:bg-[#30363d] text-[#c9d1d9] border border-[#30363d] transition-colors"
      >
        Clear Canvas
      </button>
    </div>
  );
}

