use std::sync::OnceLock;

use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use axum::{extract::State, response::IntoResponse};
use futures_util::StreamExt;
use tokio::sync::broadcast::{self, Sender};

use crate::tool::generate_playwright_spec;
use crate::{watcher_loop, SharedState};

static BROADCASTER: OnceLock<Sender<(String, String)>> = OnceLock::new();

pub fn init_broadcaster() {
    let (tx, _) = broadcast::channel::<(String, String)>(100);
    BROADCASTER.set(tx).unwrap();
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

pub async fn handle_ws(mut socket: WebSocket, state: SharedState) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Text(text) = msg {
            let reply = match text.as_str() {
                "start" => {
                    let mut st = state.lock().unwrap();
                    if st.watcher.is_none() {
                        let (tx, rx) = crossbeam_channel::bounded::<()>(1);
                        let handle = std::thread::spawn(move || watcher_loop(rx));
                        st.stop_tx = Some(tx);
                        st.watcher = Some(handle);
                        tracing::info!("Watcher startet via websocket");
                        "Watcher startet"
                    } else {
                        "Watcher kører allerede"
                    }
                },
                "stop" => {
                    let mut st = state.lock().unwrap();
                    if let Some(tx) = st.stop_tx.take() {
                        let _ = tx.send(());
                        if let Some(handle) = st.watcher.take() {
                            let _ = handle.join();
                        }
                        tracing::info!("Watcher stoppet via websocket");
                        "Watcher stoppet"
                    } else {
                        "Watcher kører ikke"
                    }
                },
                "template" => {
                    let filnavn = "eksempel.spec.ts";
                    let beskrivelse = "Bruger logger ind";
                    let trin = vec![
                        "Åbner login siden".to_string(),
                        "Indtaster brugernavn og kode".to_string(),
                        "Klikker login".to_string(),
                        "Ser forsiden vises".to_string(),
                    ];
                    let _ = generate_playwright_spec(filnavn, beskrivelse, &trin);
                    "Template done..."
                },
                "kill" => {
                    tracing::info!("Kill kaldt, lukker app via websocket");
                    // Lukker serveren efter kort delay
                    tokio::spawn(async {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        std::process::exit(0);
                    });
                    "App lukker ned..."
                },
                _ => "Ukendt kommando",
            };
            let _ = socket.send(Message::Text(reply.to_string())).await;
        }
    }
}

pub fn notify_session(session_id: &str, message: &str) {
    let _ = BROADCASTER.get().unwrap().send((session_id.to_string(), message.to_string()));
}