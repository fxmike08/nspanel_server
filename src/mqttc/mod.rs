mod model;

use bytes::Bytes;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::cards::Card;
use chrono::{FixedOffset, Timelike, Utc};
use log::{error, info, trace};
use rumqttc::v5::mqttbytes::v5::Packet::Publish;
use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::Event::Incoming;
use rumqttc::v5::{AsyncClient, EventLoop, MqttOptions};
use serde_json::Value;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::{interval, timeout, Duration};

use crate::command::{Command, Page};
use crate::config::schema::{Config, Device};
use crate::homeassitant::events::RootEvent;
use crate::mqttc::model::alarm::Alarm;
use crate::mqttc::model::screensaver::Screensaver;
use crate::utils;

type Client = (AsyncClient, EventLoop);

pub struct MqttC {
    pub config: Arc<Config>,
    pub client: Client,
    pub running: bool,
}

impl MqttC {
    pub fn new(config: Arc<Config>) -> Self {
        let mut mqttoptions = MqttOptions::new(
            "nspanel_server_rust",
            config.connectivity.mqtt.host.as_str(),
            config.connectivity.mqtt.port,
        );
        mqttoptions.set_credentials(
            &config.connectivity.mqtt.user,
            &config.connectivity.mqtt.password,
        );
        let client = AsyncClient::new(mqttoptions, 10);

        Self {
            config,
            client,
            running: false,
        }
    }

    pub async fn subscribe(
        &mut self,
        shutdown: Arc<AtomicBool>,
        channel: (
            Sender<(String, String)>,
            Arc<Mutex<Receiver<(String, String)>>>,
        ),
    ) {
        trace!("Entering in subscribe method");
        for device in self.config.devices.values() {
            let _ = self
                .client
                .0
                .subscribe(&device.mqtt.tx_topic, QoS::AtMostOnce)
                .await;
            info!(
                "Mqtt client is register to listen on topic {}",
                &device.mqtt.tx_topic
            );
        }

        let sender_to_hass = channel.0;
        let receiver_from_hass = channel.1;

        let publisher = self.client.0.clone();
        let config = self.config.clone();
        let shutdown_cloned = shutdown.clone();

        let hass_changes_future = async move {
            MqttC::send_on_event(
                publisher,
                config.as_ref(),
                shutdown_cloned,
                receiver_from_hass,
            )
            .await;
        };
        let publisher = self.client.0.clone();
        let config = self.config.clone();
        let shutdown_cloned = shutdown.clone();
        let ticker_future = async move {
            MqttC::send_periodic_message(publisher, config.as_ref(), shutdown_cloned).await;
        };

        let mqtt_handling = async move {
            while !shutdown.load(Ordering::SeqCst) {
                // MQTT event handling code goes here
                let event = timeout(Duration::from_secs(1), self.client.1.poll()).await;
                match &event {
                    Ok(Ok(e)) => {
                        match e {
                            Incoming(Publish(p)) => {
                                info!("Mqtt event {:?}", p);
                                let topic = std::str::from_utf8(p.topic.deref())
                                    .expect("Unable to get topic");
                                let payload = std::str::from_utf8(p.payload.deref())
                                    .expect("Unable to get payload");
                                let device_id = &topic[3..topic.len()];
                                let tx = self.commands_matching(device_id, payload);
                                info!("RX={:?}", tx);
                                for data in tx {
                                    let _ = self
                                        .client
                                        .0
                                        .publish("tx/nspanel-ds", QoS::ExactlyOnce, false, data)
                                        .await;
                                }
                            }
                            _ => {
                                // trace!(self.logger, "Uninteresting Mqtt event {:?}",e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Mqtt error event {:?}", e);
                    }
                    Err(_e) => {} // Timeout
                }
            }
            trace!("Exiting async loop from subscribe");
        };
        // Execute futures concurrently
        tokio::join!(ticker_future, mqtt_handling, hass_changes_future);
    }

    async fn send_on_event(
        publisher: AsyncClient,
        config: &Config,
        shutdown: Arc<AtomicBool>,
        receiver: Arc<Mutex<Receiver<(String, String)>>>,
    ) {
        while !shutdown.load(Ordering::SeqCst) {
            let message;
            // Receive messages from the shared receiver
            // Adding timeout in case config is changed
            if let Ok(result) = timeout(Duration::from_secs(5), receiver.lock().await.recv()).await
            {
                trace!("Message from Hass: {:?}", result);
                message = result;
            } else {
                continue;
            }

            if let Some((key, value)) = message {
                if let Some(device) = config.devices.get(key.as_str()) {
                    let messages = Self::parse_hass_event(config.clone(), device, value);
                    info!("Sending message to mqttc channel TX: {:?}", messages);
                    for message in messages {
                        let _ = publisher
                            .publish(
                                device.mqtt.rx_topic.clone(),
                                QoS::ExactlyOnce,
                                false,
                                Bytes::from(message),
                            )
                            .await;
                    }
                }
            } else {
                break; // Exit the loop if the channel is closed
            }
        }
        trace!("Exiting async loop from send_on_event");
    }

    fn parse_hass_event(config: Config, device: &Device, value: String) -> Vec<String> {
        use utils::DeviceState;

        let mut messages: HashMap<Card, String> = HashMap::default();

        let device_state = DeviceState::get_state(&device.id);

        // Getting RootEvent
        let json = serde_yaml::from_str::<RootEvent>(&*value).unwrap();

        // Helper closure that takes card and vec<String> and add to messages
        // We are using this closure to pass to the model methods, so we don't care about the
        // current page.
        let mut insert_message = |card: Card, result: Vec<String>| {
            let messages_to_insert = result
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>();
            messages.extend(messages_to_insert.into_iter().map(|s| (card.clone(), s)));
        };

        Screensaver::process_temperature_sensor(&config, &value, &device, &mut insert_message);
        Screensaver::process_weather(&config, device, &value, &json, &mut insert_message);
        Alarm::process_alarm_data(&config, &value, &device, &json, &mut insert_message);

        // Handle model only if are for the current page
        if let Some(&ref current_page) = device_state.page.as_ref().map(|p| &p.current) {
            messages
                .iter()
                .filter(|(c, _)| **c == device_state.page.clone().unwrap().current)
                .map(|(_, s)| s.clone())
                .collect()
        } else {
            messages.values().map(|s| s.clone()).collect::<Vec<_>>()
        }
    }

    async fn send_periodic_message(
        publisher: AsyncClient,
        config: &Config,
        shutdown: Arc<AtomicBool>,
    ) {
        let mut interval = interval(Duration::from_secs(10)); // Create an interval of seconds

        while !shutdown.load(Ordering::SeqCst) {
            trace!("Each seconds {}", 10);
            //TODO change this to send message over channel and not like how it's done now.
            for device in config.devices.values() {
                let dt = Utc::now().with_timezone(&FixedOffset::east_opt(2 * 3600).unwrap());
                let time_str = format!("time~{:0>2}:{:0>2}~", dt.hour(), dt.minute());
                let bytes = Bytes::from(time_str.into_bytes());
                let _ = publisher
                    .publish(device.mqtt.rx_topic.clone(), QoS::ExactlyOnce, false, bytes)
                    .await;
            }
            interval.tick().await;
        }
        trace!("Exiting interval loop from send_periodic_message");
    }
    fn commands_matching(&mut self, device_id: &str, payload: &str) -> Vec<Bytes> {
        let config = &self.config.clone();
        let command = Command::new(config, device_id);
        let result = serde_json::from_str(payload)
            .map(move |data: Value| {
                let d = data.as_object();
                if let Some(o) = d {
                    for (key, value) in o {
                        match key.as_str() {
                            "CustomRecv" => {
                                let tokens = value.to_string();
                                info!("Device_id [{}] Tokens {:?}", device_id, tokens);
                                if tokens.starts_with(r#""event,startup,"#) {
                                    return command.execute(Page::Startup);
                                } else if tokens.starts_with(r#""event,sleepReached,"#) {
                                    return command.execute(Page::Screensaver);
                                } else if tokens
                                    .starts_with(r#""event,buttonPress2,screensaver,bExit,"#)
                                {
                                    // Get previous page and display it.
                                    return command.execute(Page::ExistScreensaver);
                                } else if let Some(captured) =
                                    regex::Regex::new(r#"event,buttonPress2,(.*?),(bNext|bPrev)"#)
                                        .expect("Failed to parse the regex for bNext action")
                                        .captures(&tokens)
                                {
                                    if let Some(group) = captured.get(1) {
                                        // this is the group for current page
                                        if let Some(card) = config.get_adjacent_card(
                                            device_id,
                                            group.as_str(),
                                            captured.get(2).unwrap().as_str().eq("bNext"),
                                        ) {
                                            return command
                                                .execute(Page::from(card.type_.as_str()));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    vec![]
                } else {
                    vec![]
                }
            })
            .map_err(|e| {
                error!(
                    "Device_id [{}]; Unable to parse payload  error event {:?}",
                    device_id, e
                );
            });
        result.unwrap_or_else(|_| vec![])
    }
}
