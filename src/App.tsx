import { Activity, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ClientPanel } from "./components/ClientPanel";
import { EventLog } from "./components/EventLog";
import { ServerPanel } from "./components/ServerPanel";
import {
  BROADCAST_PEER_ID,
  DEFAULT_CLIENT_MESSAGE,
  DEFAULT_SERVER_CONFIG,
  DEFAULT_SERVER_MESSAGE
} from "./defaults";
import { command, isNativeRuntime, subscribeRuntimeEvents, subscribeRuntimeSnapshots } from "./lib/api";
import type { ClientStatus, RuntimeEvent, RuntimeSnapshot, ServerConfig, ServerStatus } from "./types";

const EMPTY_SERVER_STATUS: ServerStatus = {
  running: false,
  endpoint: null,
  clientCount: 0,
  config: null
};

const EMPTY_CLIENT_STATUS: ClientStatus = {
  connected: false,
  url: null
};

export default function App() {
  const [serverConfig, setServerConfig] = useState<ServerConfig>(DEFAULT_SERVER_CONFIG);
  const [serverMessage, setServerMessage] = useState(DEFAULT_SERVER_MESSAGE);
  const [clientUrl, setClientUrl] = useState("ws://127.0.0.1:9001/mock");
  const [clientMessage, setClientMessage] = useState(DEFAULT_CLIENT_MESSAGE);
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot>({
    server: EMPTY_SERVER_STATUS,
    client: EMPTY_CLIENT_STATUS,
    serverClients: [],
    events: []
  });
  const [events, setEvents] = useState<RuntimeEvent[]>([]);
  const [selectedPeerId, setSelectedPeerId] = useState(BROADCAST_PEER_ID);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const busy = busyAction !== null;
  const nativeRuntime = isNativeRuntime();

  const applySnapshot = useCallback((nextSnapshot: RuntimeSnapshot) => {
    setSnapshot(nextSnapshot);
    setEvents(nextSnapshot.events);
    if (nextSnapshot.server.config) {
      setServerConfig(nextSnapshot.server.config);
    }
    if (nextSnapshot.client.url) {
      setClientUrl(nextSnapshot.client.url);
    }
  }, []);

  const refreshSnapshot = useCallback(async () => {
    const nextSnapshot = await command<RuntimeSnapshot>("get_runtime_snapshot");
    applySnapshot(nextSnapshot);
  }, [applySnapshot]);

  useEffect(() => {
    if (!nativeRuntime) {
      setNotice("当前是浏览器预览模式，没有连接到 Tauri/Rust 运行时；服务端和客户端操作不会启动真实 WebSocket。请使用 npm run tauri dev 运行桌面应用。");
    }
    void refreshSnapshot().catch((error) => setNotice(errorMessage(error)));

    let mounted = true;
    let cleanupEvent: (() => void) | undefined;
    let cleanupSnapshot: (() => void) | undefined;

    void subscribeRuntimeEvents((event) => {
      if (!mounted) return;
      setEvents((current) => {
        if (current.some((item) => item.id === event.id)) {
          return current;
        }
        return [...current.slice(-299), event];
      });
    }).then((unlisten) => {
      cleanupEvent = unlisten;
    });

    void subscribeRuntimeSnapshots((nextSnapshot) => {
      if (mounted) {
        applySnapshot(nextSnapshot);
      }
    }).then((unlisten) => {
      cleanupSnapshot = unlisten;
    });

    return () => {
      mounted = false;
      cleanupEvent?.();
      cleanupSnapshot?.();
    };
  }, [applySnapshot, nativeRuntime, refreshSnapshot]);

  useEffect(() => {
    const peerExists = snapshot.serverClients.some((peer) => peer.id === selectedPeerId);
    if (selectedPeerId !== BROADCAST_PEER_ID && !peerExists) {
      setSelectedPeerId(BROADCAST_PEER_ID);
    }
  }, [selectedPeerId, snapshot.serverClients]);

  const serverStatus = snapshot.server;
  const clientStatus = snapshot.client;

  const metrics = useMemo(
    () => [
      { label: "运行时", value: nativeRuntime ? "Tauri" : "浏览器" },
      { label: "服务端连接", value: String(snapshot.serverClients.length) },
      { label: "事件", value: String(events.length) },
      { label: "客户端", value: clientStatus.connected ? "在线" : "离线" }
    ],
    [clientStatus.connected, events.length, nativeRuntime, snapshot.serverClients.length]
  );

  async function runAction(label: string, action: () => Promise<string | void>) {
    setBusyAction(label);
    setNotice(null);
    try {
      const message = await action();
      await refreshSnapshot();
      if (message) {
        setNotice(message);
      }
    } catch (error) {
      setNotice(errorMessage(error));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <main className="app-shell">
      <header className="app-header">
        <div className="brand-block">
          <div className="brand-mark">
            <Activity aria-hidden="true" size={22} />
          </div>
          <div>
            <p className="eyebrow">WebSocket Mock</p>
            <h1>WebSocket Mock</h1>
          </div>
        </div>
        <div className="metric-strip">
          {metrics.map((metric) => (
            <div key={metric.label} className="metric">
              <span>{metric.label}</span>
              <strong>{metric.value}</strong>
            </div>
          ))}
          <button
            type="button"
            className="tool-button"
            title="刷新状态"
            disabled={busy}
            onClick={() => void runAction("refresh", async () => refreshSnapshot())}
          >
            <RefreshCw aria-hidden="true" size={17} />
          </button>
        </div>
      </header>

      {notice ? <div className="notice">{notice}</div> : null}

      <div className="workspace-grid">
        <ServerPanel
          config={serverConfig}
          status={serverStatus}
          peers={snapshot.serverClients}
          message={serverMessage}
          selectedPeerId={selectedPeerId}
          busy={busy || !nativeRuntime}
          onConfigChange={setServerConfig}
          onMessageChange={setServerMessage}
          onSelectedPeerChange={setSelectedPeerId}
          onStart={() =>
            void runAction("start-server", async () => {
              await command<ServerStatus>("start_server", { config: serverConfig });
              return "服务端已启动";
            })
          }
          onStop={() =>
            void runAction("stop-server", async () => {
              await command<ServerStatus>("stop_server");
              return "服务端已停止";
            })
          }
          onSend={() =>
            void runAction("send-server-message", async () => {
              const result = await command<{ sent: number }>("send_server_message", {
                peerId: selectedPeerId,
                message: serverMessage
              });
              return `服务端已发送 ${result.sent} 条消息`;
            })
          }
        />

        <ClientPanel
          url={clientUrl}
          status={clientStatus}
          message={clientMessage}
          busy={busy || !nativeRuntime}
          onUrlChange={setClientUrl}
          onMessageChange={setClientMessage}
          onConnect={() =>
            void runAction("connect-client", async () => {
              await command<ClientStatus>("connect_client", { url: clientUrl });
              return "客户端已连接";
            })
          }
          onDisconnect={() =>
            void runAction("disconnect-client", async () => {
              await command<ClientStatus>("disconnect_client");
              return "客户端已断开";
            })
          }
          onSend={() =>
            void runAction("send-client-message", async () => {
              const result = await command<{ sent: number }>("send_client_message", { message: clientMessage });
              return `客户端已发送 ${result.sent} 条消息`;
            })
          }
        />
      </div>

      <EventLog events={events} onClear={() => setEvents([])} />
    </main>
  );
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "操作失败";
}
