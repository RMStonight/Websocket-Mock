use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{async_runtime::JoinHandle, AppHandle, Emitter, State};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc, oneshot, Mutex},
    time::{self, Instant, MissedTickBehavior},
};
use tokio_tungstenite::{
    accept_async,
    connect_async,
    tungstenite::Message,
};
use uuid::Uuid;

const BROADCAST_PEER_ID: &str = "__broadcast__";
const MAX_EVENTS: usize = 500;
const SERVER_PEER_HEARTBEAT_SECONDS: u64 = 15;

#[derive(Clone, Default)]
pub struct RuntimeState {
    inner: Arc<Mutex<RuntimeInner>>,
}

#[derive(Default)]
struct RuntimeInner {
    server: Option<ServerRuntime>,
    server_clients: HashMap<String, ServerPeerHandle>,
    client: Option<ClientRuntime>,
    events: VecDeque<RuntimeEvent>,
}

struct ServerRuntime {
    config: ServerConfig,
    task: JoinHandle<()>,
    shutdown: broadcast::Sender<()>,
}

struct ServerPeerHandle {
    id: String,
    address: String,
    connected_at: String,
    sender: mpsc::UnboundedSender<Message>,
}

struct ClientRuntime {
    connection_id: String,
    url: String,
    sender: mpsc::UnboundedSender<Message>,
    reader_task: JoinHandle<()>,
    writer_task: JoinHandle<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub auto_reply: bool,
    #[serde(default)]
    pub send_greeting: bool,
    pub response_template: String,
    pub greeting_template: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub server: ServerStatus,
    pub client: ClientStatus,
    pub server_clients: Vec<ServerPeer>,
    pub events: Vec<RuntimeEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub endpoint: Option<String>,
    pub client_count: usize,
    pub config: Option<ServerConfig>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    pub connected: bool,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPeer {
    pub id: String,
    pub address: String,
    pub connected_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    pub sent: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEvent {
    pub id: String,
    pub timestamp: String,
    pub source: EventSource,
    pub direction: EventDirection,
    pub level: EventLevel,
    pub title: String,
    pub payload: Option<String>,
    pub peer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum EventSource {
    Server,
    Client,
    System,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventDirection {
    Inbound,
    Outbound,
    Lifecycle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventLevel {
    Info,
    Warning,
    Error,
}

impl RuntimeState {
    async fn snapshot(&self) -> RuntimeSnapshot {
        let inner = self.inner.lock().await;
        let server_clients = sorted_server_peers(&inner.server_clients);

        RuntimeSnapshot {
            server: server_status_from_inner(&inner),
            client: client_status_from_inner(&inner),
            server_clients,
            events: inner.events.iter().cloned().collect(),
        }
    }

    async fn server_status(&self) -> ServerStatus {
        let inner = self.inner.lock().await;
        server_status_from_inner(&inner)
    }

    async fn client_status(&self) -> ClientStatus {
        let inner = self.inner.lock().await;
        client_status_from_inner(&inner)
    }
}

#[tauri::command]
pub async fn get_runtime_snapshot(state: State<'_, RuntimeState>) -> Result<RuntimeSnapshot, String> {
    Ok(state.inner().snapshot().await)
}

#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    config: ServerConfig,
) -> Result<ServerStatus, String> {
    let state = state.inner().clone();
    let config = normalize_server_config(config)?;
    validate_template_json(&config.response_template, "自动响应模板")?;

    if config.send_greeting && !config.greeting_template.trim().is_empty() {
        validate_template_json(&config.greeting_template, "欢迎消息模板")?;
    }

    {
        let mut inner = state.inner.lock().await;
        if inner.server.is_some() {
            return Err("服务端已经在运行".into());
        }
        inner.server_clients.clear();
    }

    let bind_addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|err| format!("监听 {bind_addr} 失败：{err}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|err| format!("读取监听地址失败：{err}"))?;

    let mut effective_config = config;
    effective_config.port = local_addr.port();

    let (shutdown, shutdown_rx) = broadcast::channel(32);
    let server_task = tauri::async_runtime::spawn(run_server(
        app.clone(),
        state.clone(),
        listener,
        effective_config.clone(),
        shutdown.clone(),
        shutdown_rx,
    ));

    {
        let mut inner = state.inner.lock().await;
        inner.server = Some(ServerRuntime {
            config: effective_config.clone(),
            task: server_task,
            shutdown,
        });
    }

    push_event(
        &app,
        &state,
        RuntimeEvent::new(
            EventSource::Server,
            EventDirection::Lifecycle,
            EventLevel::Info,
            "服务端已启动",
            Some(server_endpoint(&effective_config)),
            None,
        ),
    )
    .await;

    Ok(state.server_status().await)
}

#[tauri::command]
pub async fn stop_server(
    app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<ServerStatus, String> {
    let state = state.inner().clone();
    let (server, peers) = {
        let mut inner = state.inner.lock().await;
        let server = inner.server.take();
        let peers = inner
            .server_clients
            .drain()
            .map(|(_, peer)| peer.sender)
            .collect::<Vec<_>>();
        (server, peers)
    };

    if let Some(server) = server {
        let _ = server.shutdown.send(());
        for sender in peers {
            let _ = sender.send(Message::Close(None));
        }
        server.task.abort();

        push_event(
            &app,
            &state,
            RuntimeEvent::new(
                EventSource::Server,
                EventDirection::Lifecycle,
                EventLevel::Info,
                "服务端已停止",
                Some(server_endpoint(&server.config)),
                None,
            ),
        )
        .await;
    }

    Ok(state.server_status().await)
}

#[tauri::command]
pub async fn send_server_message(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    peer_id: Option<String>,
    message: String,
) -> Result<SendResult, String> {
    validate_json_text(&message, "服务端发送内容")?;
    let state = state.inner().clone();
    let selected_peer_id = peer_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);

    let targets = {
        let inner = state.inner.lock().await;
        if inner.server.is_none() {
            return Err("服务端未运行".into());
        }

        match selected_peer_id.as_deref() {
            None | Some(BROADCAST_PEER_ID) => inner
                .server_clients
                .iter()
                .map(|(id, peer)| (id.clone(), peer.sender.clone()))
                .collect::<Vec<_>>(),
            Some(id) => {
                let peer = inner
                    .server_clients
                    .get(id)
                    .ok_or_else(|| "目标连接不存在".to_string())?;
                vec![(id.to_string(), peer.sender.clone())]
            }
        }
    };

    let mut sent = 0;
    for (id, sender) in targets {
        if sender.send(Message::Text(message.clone())).is_ok() {
            sent += 1;
            push_event(
                &app,
                &state,
                RuntimeEvent::new(
                    EventSource::Server,
                    EventDirection::Outbound,
                    EventLevel::Info,
                    "服务端发送消息",
                    Some(message.clone()),
                    Some(id),
                ),
            )
            .await;
        }
    }

    Ok(SendResult { sent })
}

#[tauri::command]
pub async fn connect_client(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    url: String,
) -> Result<ClientStatus, String> {
    let url = normalize_ws_url(url)?;
    let state = state.inner().clone();

    {
        let inner = state.inner.lock().await;
        if inner.client.is_some() {
            return Err("客户端已经连接".into());
        }
    }

    let (ws_stream, _) = connect_async(url.as_str())
        .await
        .map_err(|err| format!("客户端连接失败：{err}"))?;
    let (mut writer, mut reader) = ws_stream.split();
    let (sender, mut outbound) = mpsc::unbounded_channel::<Message>();
    let connection_id = Uuid::new_v4().to_string();
    let writer_task = tauri::async_runtime::spawn(async move {
        while let Some(message) = outbound.recv().await {
            if writer.send(message).await.is_err() {
                break;
            }
        }
    });

    let reader_app = app.clone();
    let reader_state = state.clone();
    let reader_url = url.clone();
    let reader_connection_id = connection_id.clone();
    let reader_task = tauri::async_runtime::spawn(async move {
        while let Some(message) = reader.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    push_event(
                        &reader_app,
                        &reader_state,
                        RuntimeEvent::new(
                            EventSource::Client,
                            EventDirection::Inbound,
                            EventLevel::Info,
                            "客户端收到消息",
                            Some(text),
                            None,
                        ),
                    )
                    .await;
                }
                Ok(Message::Binary(bytes)) => {
                    push_event(
                        &reader_app,
                        &reader_state,
                        RuntimeEvent::new(
                            EventSource::Client,
                            EventDirection::Inbound,
                            EventLevel::Info,
                            "客户端收到二进制消息",
                            Some(format!("{} bytes", bytes.len())),
                            None,
                        ),
                    )
                    .await;
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                Ok(Message::Close(_)) => break,
                Err(err) => {
                    push_event(
                        &reader_app,
                        &reader_state,
                        RuntimeEvent::new(
                            EventSource::Client,
                            EventDirection::Lifecycle,
                            EventLevel::Error,
                            "客户端连接异常",
                            Some(err.to_string()),
                            None,
                        ),
                    )
                    .await;
                    break;
                }
                _ => {}
            }
        }

        let did_clear_client = {
            let mut inner = reader_state.inner.lock().await;
            let should_clear = inner
                .client
                .as_ref()
                .map(|client| client.connection_id.as_str() == reader_connection_id.as_str())
                .unwrap_or(false);
            if should_clear {
                inner.client = None;
            }
            should_clear
        };

        if did_clear_client {
            push_event(
                &reader_app,
                &reader_state,
                RuntimeEvent::new(
                    EventSource::Client,
                    EventDirection::Lifecycle,
                    EventLevel::Info,
                    "客户端已断开",
                    Some(reader_url),
                    None,
                ),
            )
            .await;
        }
    });

    {
        let mut inner = state.inner.lock().await;
        inner.client = Some(ClientRuntime {
            connection_id,
            url: url.clone(),
            sender,
            reader_task,
            writer_task,
        });
    }

    push_event(
        &app,
        &state,
        RuntimeEvent::new(
            EventSource::Client,
            EventDirection::Lifecycle,
            EventLevel::Info,
            "客户端已连接",
            Some(url),
            None,
        ),
    )
    .await;

    Ok(state.client_status().await)
}

#[tauri::command]
pub async fn disconnect_client(
    app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<ClientStatus, String> {
    let state = state.inner().clone();
    let client = {
        let mut inner = state.inner.lock().await;
        inner.client.take()
    };

    if let Some(client) = client {
        let client_url = client.url.clone();
        let _ = client.sender.send(Message::Close(None));
        drop(client.sender);
        drop(client.reader_task);
        drop(client.writer_task);

        push_event(
            &app,
            &state,
            RuntimeEvent::new(
                EventSource::Client,
                EventDirection::Lifecycle,
                EventLevel::Info,
                "客户端已断开",
                Some(client_url),
                None,
            ),
        )
        .await;
    }

    Ok(state.client_status().await)
}

#[tauri::command]
pub async fn send_client_message(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    message: String,
) -> Result<SendResult, String> {
    validate_json_text(&message, "客户端发送内容")?;
    let state = state.inner().clone();
    let sender = {
        let inner = state.inner.lock().await;
        inner
            .client
            .as_ref()
            .map(|client| client.sender.clone())
            .ok_or_else(|| "客户端未连接".to_string())?
    };

    sender
        .send(Message::Text(message.clone()))
        .map_err(|_| "客户端发送失败".to_string())?;

    push_event(
        &app,
        &state,
        RuntimeEvent::new(
            EventSource::Client,
            EventDirection::Outbound,
            EventLevel::Info,
            "客户端发送消息",
            Some(message),
            None,
        ),
    )
    .await;

    Ok(SendResult { sent: 1 })
}

async fn run_server(
    app: AppHandle,
    state: RuntimeState,
    listener: TcpListener,
    config: ServerConfig,
    shutdown: broadcast::Sender<()>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, address)) => {
                        let peer_app = app.clone();
                        let peer_state = state.clone();
                        let peer_config = config.clone();
                        let peer_shutdown_rx = shutdown.subscribe();
                        tauri::async_runtime::spawn(handle_server_peer(
                            peer_app,
                            peer_state,
                            stream,
                            address.to_string(),
                            peer_config,
                            peer_shutdown_rx,
                        ));
                    }
                    Err(err) => {
                        push_event(
                            &app,
                            &state,
                            RuntimeEvent::new(
                                EventSource::Server,
                                EventDirection::Lifecycle,
                                EventLevel::Error,
                                "服务端接收连接失败",
                                Some(err.to_string()),
                                None,
                            ),
                        )
                        .await;
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_server_peer(
    app: AppHandle,
    state: RuntimeState,
    stream: tokio::net::TcpStream,
    address: String,
    config: ServerConfig,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(stream) => stream,
        Err(err) => {
            push_event(
                &app,
                &state,
                RuntimeEvent::new(
                    EventSource::Server,
                    EventDirection::Lifecycle,
                    EventLevel::Warning,
                    "WebSocket 握手失败",
                    Some(format!("{address}: {err}")),
                    None,
                ),
            )
            .await;
            return;
        }
    };

    let peer_id = Uuid::new_v4().to_string();
    let connected_at = Utc::now().to_rfc3339();
    let (mut writer, mut reader) = ws_stream.split();
    let (sender, mut outbound) = mpsc::unbounded_channel::<Message>();

    {
        let mut inner = state.inner.lock().await;
        inner.server_clients.insert(
            peer_id.clone(),
            ServerPeerHandle {
                id: peer_id.clone(),
                address: address.clone(),
                connected_at: connected_at.clone(),
                sender: sender.clone(),
            },
        );
    }

    push_event(
        &app,
        &state,
        RuntimeEvent::new(
            EventSource::Server,
            EventDirection::Lifecycle,
            EventLevel::Info,
            "客户端接入服务端",
            Some(address.clone()),
            Some(peer_id.clone()),
        ),
    )
    .await;

    let (writer_done_tx, mut writer_done_rx) = oneshot::channel::<()>();
    let writer_task = tauri::async_runtime::spawn(async move {
        while let Some(message) = outbound.recv().await {
            if writer.send(message).await.is_err() {
                break;
            }
        }
        let _ = writer_done_tx.send(());
    });

    if config.send_greeting && !config.greeting_template.trim().is_empty() {
        match render_template(&config.greeting_template, "", &peer_id, None) {
            Ok(greeting) => {
                if sender.send(Message::Text(greeting.clone())).is_ok() {
                    push_event(
                        &app,
                        &state,
                        RuntimeEvent::new(
                            EventSource::Server,
                            EventDirection::Outbound,
                            EventLevel::Info,
                            "服务端发送欢迎消息",
                            Some(greeting),
                            Some(peer_id.clone()),
                        ),
                    )
                    .await;
                }
            }
            Err(err) => {
                push_event(
                    &app,
                    &state,
                    RuntimeEvent::new(
                        EventSource::Server,
                        EventDirection::Lifecycle,
                        EventLevel::Error,
                        "欢迎消息模板渲染失败",
                        Some(err),
                        Some(peer_id.clone()),
                    ),
                )
                .await;
            }
        }
    }

    let heartbeat_interval = Duration::from_secs(SERVER_PEER_HEARTBEAT_SECONDS);
    let mut heartbeat = time::interval_at(Instant::now() + heartbeat_interval, heartbeat_interval);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            shutdown = shutdown_rx.recv() => {
                match shutdown {
                    Ok(_) => {
                        let _ = sender.send(Message::Close(None));
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            _ = &mut writer_done_rx => {
                push_event(
                    &app,
                    &state,
                    RuntimeEvent::new(
                        EventSource::Server,
                        EventDirection::Lifecycle,
                        EventLevel::Warning,
                        "服务端写入通道已关闭",
                        Some(address.clone()),
                        Some(peer_id.clone()),
                    ),
                )
                .await;
                break;
            }
            _ = heartbeat.tick() => {
                if sender.send(Message::Ping(Vec::new())).is_err() {
                    break;
                }
            }
            inbound = reader.next() => {
                match inbound {
                    Some(Ok(Message::Text(text))) => {
                        let request_id = Uuid::new_v4().to_string();
                        push_event(
                            &app,
                            &state,
                            RuntimeEvent::new(
                                EventSource::Server,
                                EventDirection::Inbound,
                                EventLevel::Info,
                                "服务端收到消息",
                                Some(text.clone()),
                                Some(peer_id.clone()),
                            ),
                        )
                        .await;

                        if config.auto_reply {
                            match render_template(&config.response_template, &text, &peer_id, Some(&request_id)) {
                                Ok(reply) => {
                                    if sender.send(Message::Text(reply.clone())).is_ok() {
                                        push_event(
                                            &app,
                                            &state,
                                            RuntimeEvent::new(
                                                EventSource::Server,
                                                EventDirection::Outbound,
                                                EventLevel::Info,
                                                "服务端自动响应",
                                                Some(reply),
                                                Some(peer_id.clone()),
                                            ),
                                        )
                                        .await;
                                    }
                                }
                                Err(err) => {
                                    push_event(
                                        &app,
                                        &state,
                                        RuntimeEvent::new(
                                            EventSource::Server,
                                            EventDirection::Lifecycle,
                                            EventLevel::Error,
                                            "自动响应模板渲染失败",
                                            Some(err),
                                            Some(peer_id.clone()),
                                        ),
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        push_event(
                            &app,
                            &state,
                            RuntimeEvent::new(
                                EventSource::Server,
                                EventDirection::Inbound,
                                EventLevel::Info,
                                "服务端收到二进制消息",
                                Some(format!("{} bytes", bytes.len())),
                                Some(peer_id.clone()),
                            ),
                        )
                        .await;
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        let _ = sender.send(Message::Pong(bytes));
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        let _ = sender.send(Message::Close(frame));
                        break;
                    }
                    None => {
                        break;
                    }
                    Some(Err(err)) => {
                        push_event(
                            &app,
                            &state,
                            RuntimeEvent::new(
                                EventSource::Server,
                                EventDirection::Lifecycle,
                                EventLevel::Warning,
                                "服务端连接异常",
                                Some(err.to_string()),
                                Some(peer_id.clone()),
                            ),
                        )
                        .await;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    {
        let mut inner = state.inner.lock().await;
        inner.server_clients.remove(&peer_id);
    }
    drop(sender);
    let _ = time::timeout(Duration::from_millis(500), writer_task).await;

    push_event(
        &app,
        &state,
        RuntimeEvent::new(
            EventSource::Server,
            EventDirection::Lifecycle,
            EventLevel::Info,
            "客户端离开服务端",
            Some(address),
            Some(peer_id),
        ),
    )
    .await;
}

async fn push_event(app: &AppHandle, state: &RuntimeState, event: RuntimeEvent) {
    let snapshot = {
        let mut inner = state.inner.lock().await;
        if inner.events.len() >= MAX_EVENTS {
            inner.events.pop_front();
        }
        inner.events.push_back(event.clone());

        RuntimeSnapshot {
            server: server_status_from_inner(&inner),
            client: client_status_from_inner(&inner),
            server_clients: sorted_server_peers(&inner.server_clients),
            events: inner.events.iter().cloned().collect(),
        }
    };

    let _ = app.emit("runtime-event", event);
    let _ = app.emit("runtime-snapshot", snapshot);
}

impl RuntimeEvent {
    fn new(
        source: EventSource,
        direction: EventDirection,
        level: EventLevel,
        title: impl Into<String>,
        payload: Option<String>,
        peer_id: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            source,
            direction,
            level,
            title: title.into(),
            payload,
            peer_id,
        }
    }
}

fn server_status_from_inner(inner: &RuntimeInner) -> ServerStatus {
    let config = inner.server.as_ref().map(|server| server.config.clone());
    ServerStatus {
        running: inner.server.is_some(),
        endpoint: config.as_ref().map(server_endpoint),
        client_count: inner.server_clients.len(),
        config,
    }
}

fn client_status_from_inner(inner: &RuntimeInner) -> ClientStatus {
    ClientStatus {
        connected: inner.client.is_some(),
        url: inner.client.as_ref().map(|client| client.url.clone()),
    }
}

fn sorted_server_peers(peers: &HashMap<String, ServerPeerHandle>) -> Vec<ServerPeer> {
    let mut peers = peers
        .values()
        .map(|peer| ServerPeer {
            id: peer.id.clone(),
            address: peer.address.clone(),
            connected_at: peer.connected_at.clone(),
        })
        .collect::<Vec<_>>();
    peers.sort_by(|a, b| a.connected_at.cmp(&b.connected_at));
    peers
}

fn normalize_server_config(mut config: ServerConfig) -> Result<ServerConfig, String> {
    config.host = config.host.trim().to_string();
    config.path = config.path.trim().to_string();
    config.response_template = config.response_template.trim().to_string();
    config.greeting_template = config.greeting_template.trim().to_string();

    if config.host.is_empty() {
        return Err("监听地址不能为空".into());
    }
    if config.port == 0 {
        return Err("端口必须大于 0".into());
    }
    if config.path.is_empty() {
        config.path = "/".into();
    }
    if !config.path.starts_with('/') {
        config.path = format!("/{}", config.path);
    }
    if config.response_template.is_empty() {
        return Err("自动响应模板不能为空".into());
    }

    Ok(config)
}

fn normalize_ws_url(url: String) -> Result<String, String> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("WebSocket 地址不能为空".into());
    }
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        return Err("WebSocket 地址必须以 ws:// 或 wss:// 开头".into());
    }
    Ok(url)
}

fn server_endpoint(config: &ServerConfig) -> String {
    format!("ws://{}:{}{}", config.host, config.port, config.path)
}

fn validate_json_text(text: &str, label: &str) -> Result<(), String> {
    serde_json::from_str::<Value>(text)
        .map(|_| ())
        .map_err(|err| format!("{label}不是有效 JSON：{err}"))
}

fn validate_template_json(template: &str, label: &str) -> Result<(), String> {
    let rendered = render_template(template, r#"{"hello":"world"}"#, "preview-peer", Some("preview-request"))?;
    serde_json::from_str::<Value>(&rendered)
        .map(|_| ())
        .map_err(|err| format!("{label}不是有效 JSON 模板：{err}"))
}

fn render_template(
    template: &str,
    inbound: &str,
    peer_id: &str,
    request_id: Option<&str>,
) -> Result<String, String> {
    let inbound_value =
        serde_json::from_str::<Value>(inbound).unwrap_or_else(|_| Value::String(inbound.to_string()));
    let json_message = serde_json::to_string(&inbound_value)
        .map_err(|err| format!("请求消息序列化失败：{err}"))?;
    let timestamp = Utc::now().to_rfc3339();
    let request_id = request_id.unwrap_or("");
    let rendered = template
        .replace("{{message}}", &json_string_content(inbound)?)
        .replace("{{jsonMessage}}", &json_message)
        .replace("{{peerId}}", &json_string_content(peer_id)?)
        .replace("{{requestId}}", &json_string_content(request_id)?)
        .replace("{{timestamp}}", &json_string_content(&timestamp)?);

    serde_json::from_str::<Value>(&rendered)
        .map_err(|err| format!("渲染后的 JSON 无效：{err}"))?;

    Ok(rendered)
}

fn json_string_content(value: &str) -> Result<String, String> {
    let encoded = serde_json::to_string(value).map_err(|err| err.to_string())?;
    Ok(encoded
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(&encoded)
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_escapes_inbound_text_inside_json_string() {
        let template = r#"{"echo":"{{message}}","peer":"{{peerId}}"}"#;
        let rendered = render_template(template, r#"{"name":"A \"quoted\" user"}"#, "peer-1", None)
            .expect("template should render");
        let parsed: Value = serde_json::from_str(&rendered).expect("rendered JSON should parse");

        assert_eq!(parsed["echo"], r#"{"name":"A \"quoted\" user"}"#);
        assert_eq!(parsed["peer"], "peer-1");
    }

    #[test]
    fn template_can_embed_inbound_json_as_value() {
        let template = r#"{"echo":{{jsonMessage}}}"#;
        let rendered =
            render_template(template, r#"{"type":"ping"}"#, "peer-1", Some("request-1")).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(parsed["echo"]["type"], "ping");
    }

    #[test]
    fn normalizes_path_with_leading_slash() {
        let config = normalize_server_config(ServerConfig {
            host: "127.0.0.1".into(),
            port: 9001,
            path: "mock".into(),
            auto_reply: true,
            send_greeting: false,
            response_template: r#"{"ok":true}"#.into(),
            greeting_template: String::new(),
        })
        .unwrap();

        assert_eq!(config.path, "/mock");
    }
}
