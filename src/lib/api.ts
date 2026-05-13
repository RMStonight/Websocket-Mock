import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  RuntimeEvent,
  RuntimeSnapshot,
} from "../types";

type CommandArgs = Record<string, unknown> | undefined;

export function isNativeRuntime(): boolean {
  const hasTauriInvoke =
    typeof window !== "undefined" &&
    typeof (window as unknown as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__?.invoke ===
      "function";

  return isTauri() || hasTauriInvoke;
}

export async function command<T>(name: string, args?: CommandArgs): Promise<T> {
  if (isNativeRuntime()) {
    return invoke<T>(name, args);
  }

  return browserFallbackCommand<T>(name);
}

export async function subscribeRuntimeEvents(
  handler: (event: RuntimeEvent) => void
): Promise<() => void> {
  if (isNativeRuntime()) {
    const unlisten = await listen<RuntimeEvent>("runtime-event", (event) => handler(event.payload));
    return unlisten;
  }

  return () => undefined;
}

export async function subscribeRuntimeSnapshots(
  handler: (snapshot: RuntimeSnapshot) => void
): Promise<() => void> {
  if (isNativeRuntime()) {
    const unlisten = await listen<RuntimeSnapshot>("runtime-snapshot", (event) => handler(event.payload));
    return unlisten;
  }

  return () => undefined;
}

async function browserFallbackCommand<T>(name: string): Promise<T> {
  if (name === "get_runtime_snapshot") {
    return {
      server: {
        running: false,
        endpoint: null,
        clientCount: 0,
        config: null
      },
      client: {
        connected: false,
        url: null
      },
      serverClients: [],
      events: []
    } as T;
  }

  throw new Error("当前页面没有连接到 Tauri/Rust 运行时，无法启动真实 WebSocket 服务或客户端。请使用 npm run tauri dev 启动桌面应用。");
}
