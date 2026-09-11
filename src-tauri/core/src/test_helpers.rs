//! Shared test helpers for defguard-client-core tests.
//!
//! Provides a controllable WebSocket stub for MFA mobile-approve tests,
//! eliminating the need for Docker or external services.

use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// Command to control the WebSocket stub's behavior after a client connects.
pub enum WsStubCommand {
    /// Send a text frame to the connected client.
    SendMessage(String),
    /// Close the WebSocket connection gracefully.
    Close,
}

/// A controllable WebSocket stub for testing MFA mobile-approve flows.
///
/// Binds to a random port on localhost.  The test connects to [`WebSocketStub::addr`],
/// then sends [`WsStubCommand`] values through [`WebSocketStub::tx`] to control
/// what frames the stub emits.
pub struct WebSocketStub {
    pub addr: SocketAddr,
    pub tx: UnboundedSender<WsStubCommand>,
}

/// Start a controllable WebSocket stub on a random localhost port.
///
/// The returned [`WebSocketStub`] spawns a Tokio task that accepts exactly one
/// TCP connection and upgrades it to a WebSocket.  After the upgrade, the task
/// waits for commands on the returned `tx` sender.
///
/// # Panics
///
/// Panics if the Tokio runtime is not available.
pub async fn start_ws_stub() -> WebSocketStub {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind WebSocket stub");
    let addr = listener.local_addr().expect("Failed to get local address");
    let (tx, mut rx) = unbounded_channel::<WsStubCommand>();

    tokio::spawn(async move {
        // Accept a single connection.
        let (stream, _peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(_) => return,
        };

        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(_) => return,
        };

        let (mut write, mut _read) = ws_stream.split();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                WsStubCommand::SendMessage(text) => {
                    let _ = write.send(Message::Text(text.into())).await;
                }
                WsStubCommand::Close => {
                    let _ = write.close().await;
                    return;
                }
            }
        }
    });

    WebSocketStub { addr, tx }
}
