'use client';

import { useState, useCallback, useRef, useEffect } from 'react';
import type { WasmModule } from './useWasm';

export interface LogEntry {
  id: string;
  time: string;
  source: 'JS' | 'Rust' | 'WS';
  message: string;
}

export interface UseWebSocketReturn {
  isConnected: boolean;
  logs: LogEntry[];
  packetCount: number;
  connect: () => void;
  disconnect: () => void;
  sendTest: () => void;
  clearLogs: () => void;
  addLog: (source: LogEntry['source'], message: string) => void;
}

const WS_URL = 'ws://localhost:8080/ws';

export function useWebSocket(wasm: WasmModule | null): UseWebSocketReturn {
  const [isConnected, setIsConnected] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [packetCount, setPacketCount] = useState(0);
  const wsRef = useRef<WebSocket | null>(null);
  const logIdRef = useRef(0);

  const addLog = useCallback((source: LogEntry['source'], message: string) => {
    const entry: LogEntry = {
      id: `log-${logIdRef.current++}`,
      time: new Date().toLocaleTimeString('ja-JP'),
      source,
      message,
    };
    setLogs((prev) => [...prev.slice(-99), entry]); // Keep last 100 logs
  }, []);

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.close();
      return;
    }

    addLog('JS', `Connecting to ${WS_URL}...`);
    
    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      addLog('WS', 'Connection established!');
      setIsConnected(true);
    };

    ws.onmessage = async (event) => {
      if (!wasm) return;

      if (event.data instanceof Blob || event.data instanceof ArrayBuffer) {
        const buffer = event.data instanceof Blob 
          ? await event.data.arrayBuffer() 
          : event.data;
        const bytes = new Uint8Array(buffer);
        const count = bytes.length / 8;
        
        wasm.handle_binary(bytes);
        setPacketCount((prev) => prev + count);
        addLog('JS', `Received ${count} packets (binary)`);
      } else {
        wasm.handle_message(event.data);
        addLog('JS', `Received message: ${event.data.slice(0, 50)}...`);
      }
    };

    ws.onclose = () => {
      addLog('WS', 'Connection closed');
      setIsConnected(false);
    };

    ws.onerror = (error) => {
      addLog('WS', `Error: ${(error as ErrorEvent).message || 'Connection failed'}`);
    };
  }, [wasm, addLog]);

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  }, []);

  const sendTest = useCallback(() => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;

    const packetCount = 100;
    const buffer = new ArrayBuffer(packetCount * 8);
    const view = new DataView(buffer);

    for (let i = 0; i < packetCount; i++) {
      const offset = i * 8;
      view.setUint32(offset, i, true);
      view.setUint16(offset + 4, Math.floor(Math.random() * 65535), true);
      view.setUint16(offset + 6, Math.floor(Math.random() * 65535), true);
    }

    wsRef.current.send(buffer);
    addLog('JS', `Sent ${packetCount} test packets (binary format)`);
  }, [addLog]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      wsRef.current?.close();
    };
  }, []);

  return {
    isConnected,
    logs,
    packetCount,
    connect,
    disconnect,
    sendTest,
    clearLogs,
    addLog,
  };
}

