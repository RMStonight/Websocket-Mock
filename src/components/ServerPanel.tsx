import { Copy, Play, Send, Server, Square, Users } from "lucide-react";
import { BROADCAST_PEER_ID } from "../defaults";
import { makeEndpoint, validateJson, validateJsonTemplate } from "../lib/json";
import type { ServerConfig, ServerPeer, ServerStatus } from "../types";
import { JsonEditor } from "./JsonEditor";
import { StatusPill } from "./StatusPill";

interface ServerPanelProps {
  config: ServerConfig;
  status: ServerStatus;
  peers: ServerPeer[];
  message: string;
  selectedPeerId: string;
  busy: boolean;
  onConfigChange: (config: ServerConfig) => void;
  onMessageChange: (value: string) => void;
  onSelectedPeerChange: (value: string) => void;
  onStart: () => void;
  onStop: () => void;
  onSend: () => void;
}

export function ServerPanel({
  config,
  status,
  peers,
  message,
  selectedPeerId,
  busy,
  onConfigChange,
  onMessageChange,
  onSelectedPeerChange,
  onStart,
  onStop,
  onSend
}: ServerPanelProps) {
  const endpoint = status.endpoint ?? makeEndpoint(config.host, config.port, config.path);
  const responseValid = validateJsonTemplate(config.responseTemplate).valid;
  const greetingValid =
    !config.sendGreeting ||
    !config.greetingTemplate.trim() ||
    validateJsonTemplate(config.greetingTemplate).valid;
  const pushValid = validateJson(message).valid;

  return (
    <section className="panel server-panel" aria-label="模拟服务端">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Mock Server</p>
          <h2>
            <Server aria-hidden="true" size={20} />
            模拟服务端
          </h2>
        </div>
        <StatusPill active={status.running} activeText="运行中" inactiveText="未启动" />
      </div>

      <div className="endpoint-strip">
        <code>{endpoint}</code>
        <button
          type="button"
          className="tool-button"
          title="复制地址"
          onClick={() => void navigator.clipboard?.writeText(endpoint)}
        >
          <Copy aria-hidden="true" size={16} />
        </button>
      </div>

      <div className="form-grid three">
        <label>
          <span>Host</span>
          <input
            value={config.host}
            disabled={status.running}
            onChange={(event) => onConfigChange({ ...config, host: event.target.value })}
          />
        </label>
        <label>
          <span>Port</span>
          <input
            type="number"
            min={1}
            max={65535}
            value={config.port}
            disabled={status.running}
            onChange={(event) => onConfigChange({ ...config, port: Number(event.target.value) })}
          />
        </label>
        <label>
          <span>Path</span>
          <input
            value={config.path}
            disabled={status.running}
            onChange={(event) => onConfigChange({ ...config, path: event.target.value })}
          />
        </label>
      </div>

      <label className="toggle-line">
        <input
          type="checkbox"
          checked={config.autoReply}
          disabled={status.running}
          onChange={(event) => onConfigChange({ ...config, autoReply: event.target.checked })}
        />
        <span>自动响应</span>
      </label>

      <JsonEditor
        label="自动响应 JSON"
        value={config.responseTemplate}
        template
        disabled={status.running}
        onChange={(value) => onConfigChange({ ...config, responseTemplate: value })}
      />

      <label className="toggle-line">
        <input
          type="checkbox"
          checked={config.sendGreeting}
          disabled={status.running}
          onChange={(event) => onConfigChange({ ...config, sendGreeting: event.target.checked })}
        />
        <span>连接后发送欢迎消息</span>
      </label>

      <JsonEditor
        label="连接欢迎 JSON"
        value={config.greetingTemplate}
        template
        rows={6}
        disabled={status.running || !config.sendGreeting}
        onChange={(value) => onConfigChange({ ...config, greetingTemplate: value })}
      />

      <div className="button-row">
        {status.running ? (
          <button type="button" className="primary danger" disabled={busy} onClick={onStop}>
            <Square aria-hidden="true" size={16} />
            停止
          </button>
        ) : (
          <button
            type="button"
            className="primary"
            disabled={busy || !responseValid || !greetingValid}
            onClick={onStart}
          >
            <Play aria-hidden="true" size={16} />
            启动
          </button>
        )}
      </div>

      <div className="subsection">
        <div className="subsection-title">
          <Users aria-hidden="true" size={17} />
          连接 {peers.length}
        </div>
        <select value={selectedPeerId} onChange={(event) => onSelectedPeerChange(event.target.value)}>
          <option value={BROADCAST_PEER_ID}>全部连接</option>
          {peers.map((peer) => (
            <option key={peer.id} value={peer.id}>
              {peer.address}
            </option>
          ))}
        </select>
      </div>

      <JsonEditor label="服务端推送 JSON" value={message} rows={7} onChange={onMessageChange} />
      <button
        type="button"
        className="secondary"
        disabled={busy || !status.running || peers.length === 0 || !pushValid}
        onClick={onSend}
      >
        <Send aria-hidden="true" size={16} />
        发送
      </button>
    </section>
  );
}
