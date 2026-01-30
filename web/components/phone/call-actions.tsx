import { PhoneOff, Pause, Play, ArrowRightLeft } from "lucide-react";
import { Button } from "../ui/button";
import { CallState } from "../../lib/dialer-types";

interface CallActionsProps {
  state: CallState;
  onHangup: () => void;
  onHold: () => void;
  onResume: () => void;
  onTransfer: () => void;
  onNewCall: () => void;
}

export function CallActions({
  state,
  onHangup,
  onHold,
  onResume,
  onTransfer,
  onNewCall,
}: CallActionsProps) {
  if (state.status === "Ended") {
    return (
      <Button onClick={onNewCall} className="w-full">
        New Call
      </Button>
    );
  }

  return (
    <div className="flex flex-wrap gap-2">
      <Button variant="destructive" onClick={onHangup}>
        <PhoneOff className="mr-2 h-4 w-4" />
        Hangup
      </Button>

      {state.status === "Answered" && (
        <>
          <Button variant="secondary" onClick={onHold}>
            <Pause className="mr-2 h-4 w-4" />
            Hold
          </Button>
          <Button variant="outline" onClick={onTransfer}>
            <ArrowRightLeft className="mr-2 h-4 w-4" />
            Transfer
          </Button>
        </>
      )}

      {state.status === "Held" && (
        <>
          <Button variant="secondary" onClick={onResume}>
            <Play className="mr-2 h-4 w-4" />
            Resume
          </Button>
          <Button variant="outline" onClick={onTransfer}>
            <ArrowRightLeft className="mr-2 h-4 w-4" />
            Transfer
          </Button>
        </>
      )}
    </div>
  );
}
