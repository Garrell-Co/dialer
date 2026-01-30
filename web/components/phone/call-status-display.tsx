import { useEffect, useRef, useState } from "react";
import { Badge } from "../ui/badge";
import { CallState } from "../../lib/dialer-types";

interface CallStatusDisplayProps {
  callId: string;
  state: CallState;
}

function statusLabel(state: CallState): string {
  switch (state.status) {
    case "Ringing":
      return "Ringing";
    case "Answered":
      return "In Call";
    case "Held":
      return "On Hold";
    case "Ended":
      return state.reason ? `Ended: ${state.reason}` : "Ended";
  }
}

function statusVariant(
  state: CallState,
): "default" | "secondary" | "destructive" | "outline" {
  switch (state.status) {
    case "Ringing":
      return "outline";
    case "Answered":
      return "default";
    case "Held":
      return "secondary";
    case "Ended":
      return "destructive";
  }
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function CallStatusDisplay({ callId, state }: CallStatusDisplayProps) {
  const [elapsed, setElapsed] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (state.status === "Answered" || state.status === "Held") {
      intervalRef.current = setInterval(() => {
        setElapsed((prev) => prev + 1);
      }, 1000);
    } else {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (state.status === "Ended") {
        // keep final elapsed, don't reset
      } else {
        setElapsed(0);
      }
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [state.status]);

  return (
    <div className="flex flex-col items-center gap-2">
      <Badge variant={statusVariant(state)}>{statusLabel(state)}</Badge>
      {(state.status === "Answered" ||
        state.status === "Held" ||
        state.status === "Ended") && (
        <span className="text-sm text-muted-foreground font-mono">
          {formatDuration(elapsed)}
        </span>
      )}
      <span className="text-xs text-muted-foreground truncate max-w-[200px]">
        {callId}
      </span>
    </div>
  );
}
