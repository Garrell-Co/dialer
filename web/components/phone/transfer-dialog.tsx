import { ArrowRightLeft } from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Separator } from "../ui/separator";

interface TransferDialogProps {
  transferTarget: string;
  onTransferTargetChange: (value: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
}

export function TransferDialog({
  transferTarget,
  onTransferTargetChange,
  onConfirm,
  onCancel,
}: TransferDialogProps) {
  return (
    <div className="grid gap-3">
      <Separator />
      <Label htmlFor="transfer-target">Transfer to</Label>
      <Input
        id="transfer-target"
        type="tel"
        placeholder="1001"
        value={transferTarget}
        onChange={(e) => onTransferTargetChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && transferTarget.trim()) {
            onConfirm();
          }
          if (e.key === "Escape") {
            onCancel();
          }
        }}
        autoFocus
      />
      <div className="flex gap-2">
        <Button
          variant="outline"
          onClick={onCancel}
          className="flex-1"
        >
          Cancel
        </Button>
        <Button
          onClick={onConfirm}
          disabled={!transferTarget.trim()}
          className="flex-1"
        >
          <ArrowRightLeft className="mr-2 h-4 w-4" />
          Transfer
        </Button>
      </div>
    </div>
  );
}
