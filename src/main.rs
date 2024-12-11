use futures_util::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Result},
};

mod filters;
use filters::{ContentFilter, RateLimiter};

// Client state structure
#[derive(Debug, Clone)]
struct ClientState {
    connected_at: Instant,
    message_count: usize,
    last_message_time: Instant,
    bytes_transferred: usize,
}

impl ClientState {
    fn new() -> Self {
        Self {
            connected_at: Instant::now(),
            message_count: 0,
            last_message_time: Instant::now(),
            bytes_transferred: 0,
        }
    }
}

// Main server structure
struct WebSocketServer {
    clients: Arc<RwLock<HashMap<String, ClientState>>>,
    rate_limiter: RateLimiter,
    content_filter: ContentFilter,
}

impl WebSocketServer {
    fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: RateLimiter::new(10, 60), // 10 messages per minute
            content_filter: ContentFilter::new(
                1024 * 10, // 10KB max message size
                vec![
                    "spam".to_string(),
                    "abuse".to_string(),
                    "exploit".to_string(),
                ],
            ),
        }
    }

    async fn handle_connection(
        self: Arc<Self>,
        client_id: String,
        stream: TcpStream,
    ) -> Result<()> {
        println!("New client connected: {}", client_id);

        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Initialize client state
        {
            let mut clients = self.clients.write().await;
            clients.insert(client_id.clone(), ClientState::new());
        }

        // Message handling loop
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(msg) => {
                    let mut clients = self.clients.write().await;
                    let state = clients.get_mut(&client_id).unwrap();

                    // Update state
                    state.message_count += 1;
                    state.last_message_time = Instant::now();

                    // Check rate limit
                    if self
                        .rate_limiter
                        .is_rate_limited(state.message_count, state.last_message_time)
                    {
                        ws_sender
                            .send(Message::Text(
                                "Rate limit exceeded. Please wait.".to_string(),
                            ))
                            .await?;
                        continue;
                    }

                    match msg {
                        Message::Text(text) => {
                            // Apply content filter
                            if !self.content_filter.is_valid_message(&text) {
                                ws_sender
                                    .send(Message::Text(
                                        "Message rejected by content filter".to_string(),
                                    ))
                                    .await?;
                                continue;
                            }

                            state.bytes_transferred += text.len();

                            // Echo valid message
                            ws_sender
                                .send(Message::Text(format!("Valid message: {}", text)))
                                .await?;
                        }
                        Message::Binary(data) => {
                            state.bytes_transferred += data.len();
                            ws_sender.send(Message::Binary(data)).await?;
                        }
                        Message::Ping(data) => {
                            ws_sender.send(Message::Pong(data)).await?;
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
                Err(e) => {
                    eprintln!("Error from {}: {}", client_id, e);
                    break;
                }
            }
        }

        // Cleanup
        {
            let mut clients = self.clients.write().await;
            clients.remove(&client_id);
            println!("Client disconnected: {}", client_id);
        }

        Ok(())
    }

    async fn start(self: Arc<Self>, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr).await.expect("Failed to bind");
        println!("WebSocket server listening on: {}", addr);

        let mut client_counter = 0;

        while let Ok((stream, addr)) = listener.accept().await {
            let client_id = format!("client_{}", client_counter);
            client_counter += 1;

            println!("New connection from: {}", addr);

            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(client_id.clone(), stream).await {
                    eprintln!("Error handling connection {}: {}", client_id, e);
                }
            });
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = Arc::new(WebSocketServer::new());
    server.start("127.0.0.1:8080").await
}
