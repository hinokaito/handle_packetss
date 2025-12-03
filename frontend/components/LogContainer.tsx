'use client';

import { useEffect, useRef } from 'react';
import type { LogEntry } from '@/hooks/useWebSocket';

interface LogContainerProps {
  logs: LogEntry[];
}

const sourceStyles: Record<LogEntry['source'], string> = {
  JS: 'text-[#f0e68c]',
  Rust: 'text-[#dea584]',
  WS: 'text-[#a5d6ff]',
};

export function LogContainer({ logs }: LogContainerProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new logs are added
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div
      ref={containerRef}
      className="h-[200px] overflow-y-auto bg-[#0d1117] border border-[#30363d] rounded-lg p-4 text-sm font-mono"
    >
      {logs.length === 0 ? (
        <div className="text-[#8b949e] text-center py-8">
          No logs yet. Connect to the WebSocket server to see activity.
        </div>
      ) : (
        logs.map((log) => (
          <div
            key={log.id}
            className="py-1 border-b border-[#21262d] last:border-b-0"
          >
            <span className="text-[#8b949e] mr-2">{log.time}</span>
            <span className={`font-bold mr-2 ${sourceStyles[log.source]}`}>
              [{log.source}]
            </span>
            <span className="text-[#c9d1d9]">{log.message}</span>
          </div>
        ))
      )}
    </div>
  );
}

