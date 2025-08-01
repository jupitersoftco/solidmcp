//! MCP WebSocket Handler
//!
//! Handles WebSocket connections and message processing for the MCP server.

use {
    super::logging::{McpConnectionId, connection_span, log_connection_upgrade, log_connection_closed,
                     log_message_received, log_handler_error, log_response_sent, log_response_error, log_parse_error},
    super::shared::McpProtocolEngine,
    futures_util::{SinkExt, StreamExt},
    serde_json::{json, Value},
    std::sync::Arc,
    std::time::{Duration, Instant},
    tracing::{debug, error, info, Instrument},
    warp::{ws::Message, ws::WebSocket, ws::Ws, Filter, Rejection, Reply},
};

/// Main WebSocket handler for MCP connections
pub async fn handle_mcp_ws_main(ws: Ws) -> Result<impl Reply, Rejection> {
    // For now, we'll create a new shared handler for each WebSocket connection
    // In a more robust implementation, we'd pass this from the server
    let protocol_engine = Arc::new(McpProtocolEngine::new());

    Ok(ws.on_upgrade(move |websocket| async move {
        let connection_id = McpConnectionId::new();
        let span = connection_span(&connection_id);
        let start_time = Instant::now();
        
        async move {
            log_connection_upgrade(&connection_id);
            handle_mcp_ws(websocket, connection_id.clone(), protocol_engine.clone()).await;
            log_connection_closed(&connection_id, start_time.elapsed());
        }
        .instrument(span)
        .await
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
                let span = connection_span(&connection_id);
                let start_time = Instant::now();
                
                async move {
                    log_connection_upgrade(&connection_id);
                    handle_mcp_ws(websocket, connection_id.clone(), engine.clone()).await;
                    log_connection_closed(&connection_id, start_time.elapsed());
                }
                .instrument(span)
                .await
            }))
        })
}

/// Handle MCP WebSocket connection
async fn handle_mcp_ws(
    websocket: WebSocket,
    connection_id: McpConnectionId,
    protocol_engine: Arc<McpProtocolEngine>,
) {
    let (mut ws_sender, mut ws_receiver) = websocket.split();
    let session_id = format!("ws-{}", connection_id.0);

    info!(
        connection_id = %connection_id,
        session_id = %session_id,
        "Starting MCP message processing loop"
    );

    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(msg) => {
                if msg.is_text() {
                    let text = msg.to_str().unwrap_or("");
                    log_message_received("text", text.len());
                    debug!(
                        raw_json = %text,
                        "Raw MCP message received"
                    );

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
                                            error!(
                                            error = %e,
                                            "Failed to serialize response"
                                        );
                                            continue;
                                        }
                                    };
                                    debug!(
                                        raw_response = %response_text,
                                        "Raw MCP response"
                                    );

                                    if let Err(e) =
                                        ws_sender.send(Message::text(&response_text)).await
                                    {
                                        log_response_error(&e.to_string());
                                        break;
                                    }

                                    log_response_sent(response_text.len());
                                }
                                Err(e) => {
                                    log_handler_error("unknown", &e.to_string(), Duration::ZERO);

                                    // Extract the message ID from the original request for proper error response
                                    let message_id =
                                        message.get("id").cloned();

                                    // Create error response using structured error types
                                    let error_response = e.to_json_rpc_error(message_id);

                                    let error_text = match serde_json::to_string(&error_response) {
                                        Ok(text) => text,
                                        Err(e) => {
                                            error!("Failed to serialize error response: {}", e);
                                            break;
                                        }
                                    };
                                    if let Err(e) = ws_sender.send(Message::text(error_text)).await
                                    {
                                        log_response_error(&e.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log_parse_error(&e.to_string(), text);

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
                                    error!(
                                        error = %e,
                                        "Failed to serialize parse error response"
                                    );
                                    break;
                                }
                            };
                            if let Err(e) = ws_sender.send(Message::text(error_text)).await {
                                log_response_error(&e.to_string());
                                break;
                            }
                        }
                    }
                } else {
                    log_message_received("non-text", 0);
                }
            }
            Err(e) => {
                log_response_error(&e.to_string());
                break;
            }
        }
    }

    // Connection closed logging is handled by the parent span
}
