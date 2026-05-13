# WebSocket Mock

Tauri + React + Rust 跨平台 WebSocket mock 工具，内置模拟服务端与模拟客户端。

## 功能

- 启动本地 WebSocket mock 服务端
- 配置 JSON 自动响应模板，可选在连接建立后发送欢迎消息
- 查看服务端连接列表，向单个连接或全部连接推送 JSON
- 作为 WebSocket 客户端连接外部服务并发送 JSON
- 统一事件流展示连接、入站消息、出站消息和错误

## 架构

- `src/`：React 工作台，包含服务端面板、客户端面板、JSON 编辑器和事件流。
- `src/lib/api.ts`：Tauri command 调用封装；普通浏览器开发模式下会使用轻量 demo 后端。
- `src-tauri/src/runtime.rs`：Rust WebSocket 运行时，集中维护服务端、客户端、连接通道和事件缓冲。
- `ServerConfig.response_template`：当前支持 `{{message}}`、`{{jsonMessage}}`、`{{peerId}}`、`{{requestId}}`、`{{timestamp}}` 占位符。
- Rust 运行时会在连接进入、断开、写入失败或心跳失败时推送 `runtime-snapshot`，前端连接列表以该快照为准。
- 当前服务端会使用配置的 path 生成连接地址，但握手时不会硬拒绝其他 path；这是为了兼容部分调试工具的 WebSocket 握手实现。

后续扩展建议优先在 Rust 运行时中引入独立的 route/rule 层，例如按 path、消息类型、JSONPath 条件或延迟策略选择响应模板；React 侧只需要增加规则编辑界面并复用现有 command 边界。

## 开发命令

```bash
npm install
npm run dev
npm run tauri dev
npm run build
npm run test
```

> 本项目需要本机安装 Rust 工具链后才能运行 `npm run tauri dev` 或完成 Tauri 打包。
> `npm run dev` 只会启动浏览器预览，不会启动 Rust WebSocket 运行时；真实服务端和客户端能力必须在 Tauri 桌面应用中使用。
