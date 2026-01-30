"use client";

import { useCallback, useState } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { Badge } from "../ui/badge";
import { useCallState } from "../../hooks/use-call-state";
import * as api from "../../lib/dialer-api";
import { DialInput } from "./dial-input";
import { CallStatusDisplay } from "./call-status-display";
import { CallActions } from "./call-actions";
import { TransferDialog } from "./transfer-dialog";

export function PhonePanel() {
  const { currentCall, connected, clearCall } = useCallState();

  const [phoneNumber, setPhoneNumber] = useState("");
  const [isDialing, setIsDialing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showTransfer, setShowTransfer] = useState(false);
  const [transferTarget, setTransferTarget] = useState("");

  const handleDial = useCallback(async () => {
    setError(null);
    setIsDialing(true);
    try {
      await api.dial({
        destination: {
          type: "Loopback",
          extension: phoneNumber.trim(),
          context: "default",
        },
        from: "1000",
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Dial failed");
    } finally {
      setIsDialing(false);
    }
  }, [phoneNumber]);

  const handleHangup = useCallback(async () => {
    if (!currentCall) return;
    try {
      await api.hangup(currentCall.call_id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Hangup failed");
    }
  }, [currentCall]);

  const handleHold = useCallback(async () => {
    if (!currentCall) return;
    try {
      await api.hold(currentCall.call_id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Hold failed");
    }
  }, [currentCall]);

  const handleResume = useCallback(async () => {
    if (!currentCall) return;
    try {
      await api.resume(currentCall.call_id);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Resume failed");
    }
  }, [currentCall]);

  const handleTransfer = useCallback(async () => {
    if (!currentCall) return;
    try {
      await api.transfer(currentCall.call_id, {
        destination: {
          type: "Loopback",
          extension: transferTarget.trim(),
          context: "default",
        },
      });
      setShowTransfer(false);
      setTransferTarget("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Transfer failed");
    }
  }, [currentCall, transferTarget]);

  const handleNewCall = useCallback(() => {
    clearCall();
    setPhoneNumber("");
    setError(null);
    setShowTransfer(false);
    setTransferTarget("");
  }, [clearCall]);

  const isIdle = currentCall === null;

  return (
    <Card className="w-full max-w-sm">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">Softphone</CardTitle>
          <Badge variant={connected ? "default" : "destructive"}>
            {connected ? "Connected" : "Disconnected"}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="grid gap-4">
        {error && (
          <p className="text-sm text-red-500">{error}</p>
        )}

        {isIdle ? (
          <DialInput
            phoneNumber={phoneNumber}
            onPhoneNumberChange={setPhoneNumber}
            onDial={handleDial}
            isDialing={isDialing}
          />
        ) : (
          <>
            <CallStatusDisplay
              callId={currentCall.call_id}
              state={currentCall.state}
            />
            <CallActions
              state={currentCall.state}
              onHangup={handleHangup}
              onHold={handleHold}
              onResume={handleResume}
              onTransfer={() => setShowTransfer(!showTransfer)}
              onNewCall={handleNewCall}
            />
            {showTransfer && (
              <TransferDialog
                transferTarget={transferTarget}
                onTransferTargetChange={setTransferTarget}
                onConfirm={handleTransfer}
                onCancel={() => {
                  setShowTransfer(false);
                  setTransferTarget("");
                }}
              />
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
