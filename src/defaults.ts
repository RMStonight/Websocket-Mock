import type { ServerConfig } from "./types";

export const BROADCAST_PEER_ID = "__broadcast__";

export const DEFAULT_SERVER_CONFIG: ServerConfig = {
  host: "127.0.0.1",
  port: 9001,
  path: "/mock",
  autoReply: true,
  sendGreeting: false,
  responseTemplate: JSON.stringify(
    {
      type: "mock.response",
      ok: true,
      requestId: "{{requestId}}",
      receivedAt: "{{timestamp}}",
      echo: "{{message}}"
    },
    null,
    2
  ),
  greetingTemplate: JSON.stringify(
    {
      type: "mock.ready",
      message: "connected",
      peerId: "{{peerId}}",
      connectedAt: "{{timestamp}}"
    },
    null,
    2
  )
};

export const DEFAULT_CLIENT_MESSAGE = JSON.stringify(
  {
    type: "ping",
    payload: {
      from: "mock-client"
    }
  },
  null,
  2
);

export const DEFAULT_SERVER_MESSAGE = JSON.stringify(
  {
    type: "server.push",
    payload: {
      status: "ready"
    }
  },
  null,
  2
);
