mod runtime;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(runtime::RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            runtime::connect_client,
            runtime::disconnect_client,
            runtime::get_runtime_snapshot,
            runtime::send_client_message,
            runtime::send_server_message,
            runtime::start_server,
            runtime::stop_server
        ])
        .run(tauri::generate_context!())
        .expect("failed to run WebSocket Mock");
}

