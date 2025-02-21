use std::{io, process::Command, time::Duration, env};
use clap::{Parser, ValueEnum, command};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, error, debug};
use tokio::sync::mpsc;
use tokio::time::{sleep, Instant};

mod config;
mod token;
use config::{Config, Environment};
use token::TokenManager;

// Define constants for WebSocket ping/pong
const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_enum, default_value_t = EnvArg::Production)]
    env: EnvArg,

    #[arg(long)]
    custom_url: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
enum EnvArg {
    Local,
    Production,
}

#[derive(Debug, Serialize, Deserialize)]
struct WsUrlResponse {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct WsMessage {
    action: String,
    channelId: String,
    clientType: String,
    message: WsMessageContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct WsMessageContent {
    #[serde(rename = "type")]
    msg_type: String,
    id: String,
    data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlationId: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConnectionMetadata {
    version: String,
    os: String,
    working_dir: String,
    #[serde(rename = "type")]
    client_type: String,
}

/// Executes a shell command and returns its output
async fn execute_command(command: &str) -> io::Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", command])
            .output()?
    } else {
        Command::new("sh")
            .args(&["-c", command])
            .output()?
    };

    let mut result = String::new();
    if !output.stdout.is_empty() {
        result.push_str("STDOUT:\n");
        result.push_str(&String::from_utf8_lossy(&output.stdout));
        result.push_str("\n");
    }
    if !output.stderr.is_empty() {
        result.push_str("STDERR:\n");
        result.push_str(&String::from_utf8_lossy(&output.stderr));
        result.push_str("\n");
    }
    
    result.push_str(&format!("Command exited with status code: {}", 
        output.status.code().unwrap_or(-1)));
        
    Ok(result)
}

async fn get_ws_url(config: &Config, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(&config.ws_url_endpoint())
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .json::<WsUrlResponse>()
        .await?;
    
    Ok(response.url)
}

async fn handle_ws_connection(ws_url: &str, token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create metadata
    let metadata = ConnectionMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        working_dir: env::current_dir()?.to_string_lossy().into_owned(),
        client_type: "terminal".to_string(),
    };

    // URL encode the metadata JSON
    let metadata_json = serde_json::to_string(&metadata)?;
    let encoded_metadata = form_urlencoded::byte_serialize(metadata_json.as_bytes()).collect::<String>();

    // Add metadata to connection URL
    let url = format!(
        "{}?clientType=server&channelId={}&metadata={}", 
        ws_url, 
        token,
        encoded_metadata
    );

    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();
    
    info!("WebSocket connection established");

    let (tx, mut rx) = mpsc::channel(32);
    
    // Create a separate sender for pings
    let ping_tx = tx.clone();
    
    // Spawn ping sender task
    let ping_handle = tokio::spawn(async move {
        loop {
            sleep(PING_INTERVAL).await;
            if let Err(e) = ping_tx.send(Message::Ping(vec![].into())).await {
                error!(error = %e, "Failed to send ping");
                break;
            }
        }
    });

    let mut last_pong = Instant::now();

    // Spawn a task to forward messages from the channel to the WebSocket
    let forward_handle = tokio::spawn({
        async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    error!(error = %e, "Failed to forward message to WebSocket");
                    break;
                }
            }
        }
    });

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Pong(_)) => {
                debug!("Received pong");
                last_pong = Instant::now();
            }
            Ok(Message::Text(text)) => {
                // Check pong timeout
                if last_pong.elapsed() >= PONG_TIMEOUT {
                    error!("WebSocket connection timed out - no pong received");
                    break;
                }
                
                debug!(message = %text, "Received message");
                
                let ws_msg: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!(error = %e, "Failed to parse message");
                        continue;
                    }
                };

                if let Some(message) = ws_msg.get("message") {
                    if let Some(data) = message.get("data") {
                        if let Some(command) = data.get("command") {
                            let cmd_str = command.as_str().unwrap_or_default();
                            info!(command = %cmd_str, "Executing command");
                            
                            let output = execute_command(cmd_str).await;
                            let response = WsMessage {
                                action: "message".to_string(),
                                channelId: token.to_string(),
                                clientType: "server".to_string(),
                                message: WsMessageContent {
                                    msg_type: "response".to_string(),
                                    id: uuid::Uuid::new_v4().to_string(),
                                    data: serde_json::json!({
                                        "output": output.as_ref().map_or_else(|e| e.to_string(), |s| s.clone()),
                                        "error": output.err().map(|e| e.to_string()),
                                    }),
                                    correlationId: Some(message.get("id").and_then(|id| id.as_str().map(String::from)).unwrap_or_default()),
                                },
                            };

                            if let Err(e) = tx.send(Message::Text(serde_json::to_string(&response).unwrap().into())).await {
                                error!(error = %e, "Failed to send response");
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by server");
                break;
            }
            Err(e) => {
                error!(error = %e, "WebSocket error");
                break;
            }
            _ => {}
        }

        // Check for pong timeout
        if last_pong.elapsed() >= PONG_TIMEOUT {
            error!("Pong timeout - connection appears stale");
            break;
        }
    }

    // Cleanup
    ping_handle.abort();
    forward_handle.abort();
    
    Err("Connection terminated".into())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging with timestamps and log levels
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .init();

    let args = Args::parse();
    
    let env = match (args.env, args.custom_url.clone()) {
        (_, Some(url)) => Environment::Custom(url.clone()),
        (EnvArg::Local, _) => Environment::Local,
        (EnvArg::Production, _) => Environment::Production,
    };

    let config = Config::new(&env)?;
    info!(environment = %env, "CLI starting up");
    debug!(config = ?config, "Using configuration");
    
    let token_manager = TokenManager::new()?;
    let token = if let Some(existing_token) = token_manager.load_token()? {
        info!("Using existing authentication token");
        existing_token
    } else {
        let new_token = TokenManager::generate_token();
        token_manager.save_token(&new_token)?;
        info!("Generated new authentication token");
        new_token
    };

    let auth_url = config.auth_url(&token);
    println!("Please visit {} to authenticate your session", auth_url);

    loop {
        match get_ws_url(&config, &token).await {
            Ok(ws_url) => {
                info!(url = %ws_url, "WebSocket URL obtained");
                if let Err(e) = handle_ws_connection(&ws_url, &token).await {
                    error!(error = %e, "WebSocket connection error");
                    if e.to_string().contains("401") {
                        let auth_url = config.auth_url(&token);
                        println!("Please visit {} to authenticate your session", auth_url);
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to get WebSocket URL");
                if e.to_string().contains("401") {
                    let auth_url = config.auth_url(&token);
                    println!("Please visit {} to authenticate your session", auth_url);
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
