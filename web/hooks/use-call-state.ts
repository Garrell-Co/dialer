import { useCallback, useEffect, useRef, useState } from "react";
import { CallStateUpdate } from "../lib/dialer-types";

const BASE_URL =
  process.env.NEXT_PUBLIC_DIALER_API_URL || "http://localhost:3001";

function wsUrl(): string {
  return BASE_URL.replace(/^http/, "ws") + "/ws";
}

export interface UseCallStateReturn {
  currentCall: CallStateUpdate | null;
  connected: boolean;
  clearCall: () => void;
}

export function useCallState(): UseCallStateReturn {
  const [currentCall, setCurrentCall] = useState<CallStateUpdate | null>(null);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearCall = useCallback(() => setCurrentCall(null), []);

  useEffect(() => {
    let disposed = false;

    function connect() {
      if (disposed) return;

      const ws = new WebSocket(wsUrl());
      wsRef.current = ws;

      ws.onopen = () => {
        if (!disposed) setConnected(true);
      };

      ws.onclose = () => {
        if (!disposed) {
          setConnected(false);
          retryRef.current = setTimeout(connect, 2000);
        }
      };

      ws.onerror = () => {
        ws.close();
      };

      ws.onmessage = (event) => {
        try {
          const update: CallStateUpdate = JSON.parse(event.data);
          setCurrentCall(update);
        } catch {
          // ignore malformed messages
        }
      };
    }

    connect();

    return () => {
      disposed = true;
      if (retryRef.current) clearTimeout(retryRef.current);
      wsRef.current?.close();
    };
  }, []);

  return { currentCall, connected, clearCall };
}
