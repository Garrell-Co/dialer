import { DialRequest, TransferRequest } from "./dialer-types";

const BASE_URL =
  process.env.NEXT_PUBLIC_DIALER_API_URL || "http://localhost:3001";

async function request(path: string, options: RequestInit): Promise<Response> {
  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers: { "Content-Type": "application/json", ...options.headers },
  });
  if (!res.ok) {
    throw new Error(`Dialer API error: ${res.status}`);
  }
  return res;
}

export async function dial(body: DialRequest): Promise<void> {
  await request("/calls", { method: "POST", body: JSON.stringify(body) });
}

export async function hangup(callId: string): Promise<void> {
  await request(`/calls/${callId}`, { method: "DELETE" });
}

export async function hold(callId: string): Promise<void> {
  await request(`/calls/${callId}/hold`, { method: "POST" });
}

export async function resume(callId: string): Promise<void> {
  await request(`/calls/${callId}/resume`, { method: "POST" });
}

export async function transfer(
  callId: string,
  body: TransferRequest,
): Promise<void> {
  await request(`/calls/${callId}/transfer`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}
