import { Link2, Play, Send, Square } from "lucide-react";
import { validateJson } from "../lib/json";
import type { ClientStatus } from "../types";
import { JsonEditor } from "./JsonEditor";
import { StatusPill } from "./StatusPill";

interface ClientPanelProps {
  url: string;
  status: ClientStatus;
  message: string;
  busy: boolean;
  onUrlChange: (value: string) => void;
  onMessageChange: (value: string) => void;
  onConnect: () => void;
  onDisconnect: () => void;
  onSend: () => void;
}

export function ClientPanel({
  url,
  status,
  message,
  busy,
  onUrlChange,
  onMessageChange,
  onConnect,
  onDisconnect,
  onSend
}: ClientPanelProps) {
  const messageValid = validateJson(message).valid;
  const urlValid = url.startsWith("ws://") || url.startsWith("wss://");

  return (
    <section className="panel client-panel" aria-label="模拟客户端">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Mock Client</p>
          <h2>
            <Link2 aria-hidden="true" size={20} />
            模拟客户端
          </h2>
        </div>
        <StatusPill active={status.connected} activeText="已连接" inactiveText="未连接" />
      </div>

      <label className="field-wide">
        <span>URL</span>
        <input value={url} disabled={status.connected} onChange={(event) => onUrlChange(event.target.value)} />
      </label>

      <div className="button-row">
        {status.connected ? (
          <button type="button" className="primary danger" disabled={busy} onClick={onDisconnect}>
            <Square aria-hidden="true" size={16} />
            断开
          </button>
        ) : (
          <button type="button" className="primary" disabled={busy || !urlValid} onClick={onConnect}>
            <Play aria-hidden="true" size={16} />
            连接
          </button>
        )}
      </div>

      <JsonEditor label="客户端发送 JSON" value={message} rows={10} onChange={onMessageChange} />

      <button
        type="button"
        className="secondary"
        disabled={busy || !status.connected || !messageValid}
        onClick={onSend}
      >
        <Send aria-hidden="true" size={16} />
        发送
      </button>
    </section>
  );
}

