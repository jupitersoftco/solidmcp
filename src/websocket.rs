//! MCP WebSocket Handler
//!
//! Handles WebSocket connections and message processing for the MCP server.

use {
    super::logging::McpConnectionId,
    super::logging::McpDebugLogger,
    super::shared::McpProtocolEngine,
    anyhow::Result,
    futures_util::{SinkExt, StreamExt},
    serde_json::{json, Value},
    std::sync::Arc,
    tracing::{debug, error, info},
    warp::{ws::Message, ws::WebSocket, ws::Ws, Filter, Rejection, Reply},
};

/// Main WebSocket handler for MCP connections
pub async fn handle_mcp_ws_main(ws: Ws) -> Result<impl Reply, Rejection> {
    // For now, we'll create a new shared handler for each WebSocket connection
    // In a more robust implementation, we'd pass this from the server
    let protocol_engine = Arc::new(McpProtocolEngine::new());

    Ok(ws.on_upgrade(move |websocket| async move {
        let connection_id = McpConnectionId::new();
        let logger = McpDebugLogger::new(connection_id.clone());

        info!(
            "🔌 MCP WebSocket connection established: {:?}",
            connection_id
        );

        handle_mcp_ws(websocket, logger, protocol_engine.clone()).await;

        info!("🔌 MCP WebSocket connection closed: {:?}", connection_id);
    }))
}

/// WebSocket handler that accepts a protocol engine
pub fn create_ws_handler(
    protocol_engine: Arc<McpProtocolEngine>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("mcp")
        .and(warp::ws())
        .and(warp::any().map(move || protocol_engine.clone()))
        .and_then(|ws: Ws, engine: Arc<McpProtocolEngine>| async move {
            Ok::<_, Rejection>(ws.on_upgrade(move |websocket| async move {
                let connection_id = McpConnectionId::new();
                let logger = McpDebugLogger::new(connection_id.clone());

                info!(
                    "🔌 MCP WebSocket connection established: {:?}",
                    connection_id
                );

                handle_mcp_ws(websocket, logger, engine.clone()).await;

                info!("🔌 MCP WebSocket connection closed: {:?}", connection_id);
            }))
        })
}

/// Handle MCP WebSocket connection
async fn handle_mcp_ws(
    websocket: WebSocket,
    logger: McpDebugLogger,
    protocol_engine: Arc<McpProtocolEngine>,
) {
    let (mut ws_sender, mut ws_receiver) = websocket.split();
    let session_id = format!("ws-{}", logger.connection_id().0);

    info!("{}", logger.fmt_connection_start());

    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(msg) => {
                if msg.is_text() {
                    let text = msg.to_str().unwrap_or("");
                    debug!("{}", logger.fmt_message_received("text", text.len()));
                    debug!("📥 Raw MCP JSON: {}", text);

                    match serde_json::from_str::<Value>(text) {
                        Ok(message) => {
                            match protocol_engine
                                .handle_message(message.clone(), Some(session_id.clone()))
                                .await
                            {
                                Ok(response) => {
                                    let response_text = match serde_json::to_string(&response) {
                                        Ok(text) => text,
                                        Err(e) => {
                                            error!("Failed to serialize response: {}", e);
                                            continue;
                                        }
                                    };
                                    debug!("📤 Raw MCP Response: {}", response_text);

                                    if let Err(e) =
                                        ws_sender.send(Message::text(&response_text)).await
                                    {
                                        error!("{}", logger.fmt_response_error(&e.to_string()));
                                        break;
                                    }

                                    debug!("{}", logger.fmt_response_sent(response_text.len()));
                                }
                                Err(e) => {
                                    error!(
                                        "{}",
                                        logger.fmt_message_handling_error(
                                            "unknown",
                                            &e.to_string(),
                                            std::time::Duration::ZERO
                                        )
                                    );

                                    // Extract the message ID from the original request for proper error response
                                    let message_id =
                                        message.get("id").unwrap_or(&json!(null)).clone();

                                    // Create error response
                                    let error_response = json!({
                                        "jsonrpc": "2.0",
                                        "id": message_id,
                                        "error": {
                                            "code": -32603,
                                            "message": format!("Internal error: {}", e)
                                        }
                                    });

                                    let error_text = match serde_json::to_string(&error_response) {
                                        Ok(text) => text,
                                        Err(e) => {
                                            error!("Failed to serialize error response: {}", e);
                                            break;
                                        }
                                    };
                                    if let Err(e) = ws_sender.send(Message::text(error_text)).await
                                    {
                                        error!("{}", logger.fmt_response_error(&e.to_string()));
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("{}", logger.fmt_parse_error(&e.to_string(), text));

                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": -32700,
                                    "message": format!("Parse error: {}", e)
                                }
                            });

                            let error_text = match serde_json::to_string(&error_response) {
                                Ok(text) => text,
                                Err(e) => {
                                    error!("Failed to serialize parse error response: {}", e);
                                    break;
                                }
                            };
                            if let Err(e) = ws_sender.send(Message::text(error_text)).await {
                                error!("{}", logger.fmt_response_error(&e.to_string()));
                                break;
                            }
                        }
                    }
                } else {
                    debug!("{}", logger.fmt_message_received("non-text", 0));
                }
            }
            Err(e) => {
                error!("{}", logger.fmt_response_error(&e.to_string()));
                break;
            }
        }
    }

    info!("{}", logger.fmt_connection_closed());
}
