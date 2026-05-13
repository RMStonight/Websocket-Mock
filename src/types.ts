export type EventSource = "server" | "client" | "system";
export type EventDirection = "inbound" | "outbound" | "lifecycle";
export type EventLevel = "info" | "warning" | "error";

export interface ServerConfig {
  host: string;
  port: number;
  path: string;
  autoReply: boolean;
  sendGreeting: boolean;
  responseTemplate: string;
  greetingTemplate: string;
}

export interface ServerStatus {
  running: boolean;
  endpoint: string | null;
  clientCount: number;
  config: ServerConfig | null;
}

export interface ClientStatus {
  connected: boolean;
  url: string | null;
}

export interface ServerPeer {
  id: string;
  address: string;
  connectedAt: string;
}

export interface RuntimeEvent {
  id: string;
  timestamp: string;
  source: EventSource;
  direction: EventDirection;
  level: EventLevel;
  title: string;
  payload: string | null;
  peerId: string | null;
}

export interface RuntimeSnapshot {
  server: ServerStatus;
  client: ClientStatus;
  serverClients: ServerPeer[];
  events: RuntimeEvent[];
}

export interface SendResult {
  sent: number;
}
