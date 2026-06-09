use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::printer;
use crate::protocol::{IncomingMessage, OutgoingMessage, PrintType};

pub async fn run_server(config: Arc<Config>, tls_config: Arc<ServerConfig>) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .context("Invalid server address")?;

    let acceptor = TlsAcceptor::from(tls_config);
    let listener = TcpListener::bind(&addr).await.context("Failed to bind")?;

    info!("PrintBridge WebSocket server listening on wss://{}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!("Incoming connection from {}", peer_addr);
                let acceptor = acceptor.clone();
                let config = config.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, peer_addr, acceptor, config).await {
                        error!("Connection error from {}: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    acceptor: TlsAcceptor,
    config: Arc<Config>,
) -> Result<()> {
    let tls_stream = acceptor
        .accept(stream)
        .await
        .context("TLS handshake failed")?;

    let allowed_origins = config.server.allowed_origins.clone();

    let ws_stream = accept_hdr_async(tls_stream, |req: &Request, res: Response| {
        if let Some(origin) = req.headers().get("origin") {
            let origin_str = origin.to_str().unwrap_or("");
            if !allowed_origins.iter().any(|o| o == origin_str || o == "*") {
                warn!("Rejected origin: {}", origin_str);
                // Build a 403 response using tungstenite's http types
                let reject = Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Some("Origin not allowed".to_string()))
                    .unwrap();
                return Err(reject);
            }
        }
        Ok(res)
    })
    .await
    .context("WebSocket upgrade failed")?;

    info!("WebSocket connected: {}", peer);
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!("WS read error from {}: {}", peer, e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                let response = process_message(&text).await;
                let json = serde_json::to_string(&response).unwrap_or_default();
                if let Err(e) = write.send(Message::Text(json)).await {
                    error!("WS send error: {}", e);
                    break;
                }
            }
            Message::Close(_) => {
                info!("Client {} disconnected", peer);
                break;
            }
            Message::Ping(d) => {
                let _ = write.send(Message::Pong(d)).await;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn process_message(text: &str) -> OutgoingMessage {
    let incoming: IncomingMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => return OutgoingMessage::err("unknown", format!("Invalid message: {}", e)),
    };

    match incoming {
        IncomingMessage::Ping { id } => OutgoingMessage::ok(id, json!({ "pong": true })),

        IncomingMessage::Status { id } => OutgoingMessage::ok(
            id,
            json!({
                "version": env!("CARGO_PKG_VERSION"),
                "status": "running"
            }),
        ),

        IncomingMessage::ListPrinters { id } => match printer::list_printers() {
            Ok(printers) => {
                let list: Vec<_> = printers
                    .iter()
                    .map(|p| {
                        json!({
                            "name":      p.name,
                            "isDefault": p.is_default,
                            "isOnline":  p.is_online,
                        })
                    })
                    .collect();
                OutgoingMessage::ok(id, json!({ "printers": list }))
            }
            Err(e) => OutgoingMessage::err(id, format!("Failed to list printers: {}", e)),
        },

        IncomingMessage::Print { id, payload } => {
            let bytes = match payload.print_type {
                PrintType::Text => payload.data.clone().into_bytes(),
                _ => match base64::engine::general_purpose::STANDARD.decode(&payload.data) {
                    Ok(b) => b,
                    Err(e) => {
                        return OutgoingMessage::err(
                            id,
                            format!("Failed to decode base64 data: {}", e),
                        )
                    }
                },
            };

            let copies = payload.copies.unwrap_or(1).max(1);

            for copy in 0..copies {
                info!(
                    "Print copy {}/{} to '{}'",
                    copy + 1,
                    copies,
                    payload.printer
                );
                if let Err(e) = printer::print_raw(&payload.printer, &bytes) {
                    return OutgoingMessage::err(
                        id,
                        format!("Print failed on copy {}: {}", copy + 1, e),
                    );
                }
            }

            OutgoingMessage::ok(id, json!({ "printed": true, "copies": copies }))
        }
    }
}
