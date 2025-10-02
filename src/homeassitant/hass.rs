use crate::config::schema::Config;
use crate::homeassitant::events::RootEvent;
use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt, TryStreamExt};
use log::{error, info, trace};
use std::collections::HashMap;
use std::string::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub fn start_hass(
    config: Arc<Config>,
    shutdown: Arc<AtomicBool>,
    channel: (
        Sender<(String, String)>,
        Arc<Mutex<Receiver<(String, String)>>>,
    ),
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let (sender, mut receiver) = mpsc::channel::<String>(10);
        let shutdown_clone = shutdown.clone();
        let shared_map = Arc::new(RwLock::new(HashMap::new()));

        let sender_to_mqtt = channel.0;
        let receiver_from_mqtt = channel.1;

        while !shutdown_clone.load(Ordering::SeqCst) {
            let cloned_sender = sender.clone();
            if let Ok((ws_stream, _)) = connect_async(format!(
                "ws://{}:{}/api/websocket",
                config.connectivity.hass.host, config.connectivity.hass.port
            ))
            .await
            {
                let (mut write, read) = ws_stream.split();

                // Authenticate
                let _ = write
                    .send(Message::Text(
                        format!(
                            r#"{{ "type": "auth", "access_token": "{}" }}"#,
                            config.connectivity.hass.token
                        )
                        .into(),
                    ))
                    .await;

                let mut seq = 1;
                // Subscribe for entities state changes
                let b_tree_entities = config.get_entities();
                for (key, entities) in b_tree_entities {
                    //TODO call a model to obtain interested data in specific format
                    let _ = write
                        .send(Message::Text(format!(
                            r#"{{ "id": {}, "type": "subscribe_entities", "entity_ids": {:?} }}"#,
                            seq, entities
                        ).into()))
                        .await;
                    let mut map = shared_map.write().unwrap();
                    map.insert(seq.to_string(), key);
                    // increment seq for other messages
                    seq += 1;
                }

                // Clone the HashMap
                let cloned_map = shared_map.read().unwrap().clone();
                // Spawn a task to handle incoming messages
                tokio::spawn(handle_messages(
                    read,
                    cloned_sender,
                    shutdown.clone(),
                    sender_to_mqtt.clone(),
                    cloned_map,
                ));

                tokio::spawn(handle_messages_from_mqtt(
                    shutdown.clone(),
                    receiver_from_mqtt.clone(),
                ));

                // This loop listens for any reconnect signals
                while let Some(msg) = receiver.recv().await {
                    // Logic to handle received messages
                    if msg == "Reconnect" {
                        info!("HASS - reconnecting on a 5 sec interval.");
                        thread::sleep(Duration::from_secs(5));
                        break;
                    }
                    if msg == "Shutdown" {
                        info!("HASS - shutting down.");
                        break;
                    }
                }
            } else {
                error!("HASS - Failed to connect to the WebSocket server");
            }
        }
    })
}

async fn handle_messages_from_mqtt(
    shutdown: Arc<AtomicBool>,
    mqtt_msg: Arc<Mutex<Receiver<(String, String)>>>,
) {
    while !shutdown.load(Ordering::SeqCst) {
        let message;
        // Receive messages from the shared receiver
        // Adding timeout in case config is changed
        if let Ok(result) = timeout(Duration::from_secs(5), mqtt_msg.lock().await.recv()).await {
            trace!("Message from Mqtt: {:?}", result);
            message = result;
        } else {
            continue;
        }
        if let Some((key, value)) = message {
        } else {
            break; // Exit the loop if the channel is closed
        }
    }
    trace!("Exiting async loop from send_on_event");
}

pub async fn handle_messages(
    ws_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    sender: Sender<String>,
    shutdown: Arc<AtomicBool>,
    sender_to_mqtt: Sender<(String, String)>,
    shared_map_clone: HashMap<String, String>,
) {
    // Handle incoming messages
    let mut incoming = ws_stream.into_stream();
    loop {
        if shutdown.load(Ordering::SeqCst) {
            let _ = sender.send("Shutdown".to_string()).await;
            break;
        }
        match timeout(Duration::from_secs(1), incoming.next()).await {
            Ok(Some(message)) => {
                match message {
                    Ok(msg) => {
                        match msg {
                            Message::Text(txt) => {
                                info!("Received message: {}", txt);
                                if txt.contains("\"type\":\"event\"") {
                                    let json = serde_yaml::from_str::<RootEvent>(&*txt).unwrap();
                                    // info!(logger, "HASS message serde json {:?}", json);
                                    if let Some(device_id) =
                                        shared_map_clone.get(&*json.id.to_string())
                                    {
                                        let _ = sender_to_mqtt
                                            .send((device_id.clone(), txt.to_string()))
                                            .await;
                                    }
                                }

                                // Handle the received text message accordingly
                            }
                            Message::Binary(_) => {
                                // Handle binary message
                            }
                            Message::Ping(_) | Message::Pong(_) => {
                                // Handle ping/pong messages if necessary
                            }
                            Message::Close(_) => {
                                info!("HASS - Connection closed.");
                                let _ = sender.send("Reconnect".to_string()).await;
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        error!("HASS - Error receiving message: {:?}", e);
                        break;
                    }
                }
            }
            Err(_) => {} // Timeout occurred
            _ => {}
        }
    }
}
