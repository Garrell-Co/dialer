import { Phone } from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";

interface DialInputProps {
  phoneNumber: string;
  onPhoneNumberChange: (value: string) => void;
  onDial: () => void;
  isDialing: boolean;
}

export function DialInput({
  phoneNumber,
  onPhoneNumberChange,
  onDial,
  isDialing,
}: DialInputProps) {
  return (
    <div className="grid gap-3">
      <Label htmlFor="phone-number">Number</Label>
      <Input
        id="phone-number"
        type="tel"
        placeholder="1000"
        value={phoneNumber}
        onChange={(e) => onPhoneNumberChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && phoneNumber.trim()) {
            onDial();
          }
        }}
        disabled={isDialing}
      />
      <Button
        onClick={onDial}
        disabled={!phoneNumber.trim() || isDialing}
        className="w-full"
      >
        <Phone className="mr-2 h-4 w-4" />
        {isDialing ? "Dialing..." : "Dial"}
      </Button>
    </div>
  );
}
