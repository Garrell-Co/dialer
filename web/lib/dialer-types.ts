// TypeScript types mirroring Rust enums from dialer/src/

// --- CallState (serde: tag = "status") ---

export type CallState =
  | { status: "Ringing" }
  | { status: "Answered" }
  | { status: "Held" }
  | { status: "Ended"; reason?: string };

// --- DestinationType (serde: tag = "type") ---

export type DestinationType =
  | { type: "Loopback"; extension: string; context: string }
  | { type: "RegisteredUser"; user: string; domain?: string }
  | { type: "External"; destination: string }
  | { type: "Gateway"; gateway_name: string; number: string };

// --- API request bodies ---

export interface DialRequest {
  destination: DestinationType;
  from: string;
  caller_id_name?: string;
}

export interface TransferRequest {
  destination: DestinationType;
}

// --- WebSocket event ---

export interface CallStateUpdate {
  call_id: string;
  state: CallState;
}
