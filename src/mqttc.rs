use bytes::Bytes;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{FixedOffset, Timelike, Utc};
use rumqttc::v5::mqttbytes::v5::Packet::Publish;
use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::Event::Incoming;
use rumqttc::v5::{AsyncClient, EventLoop, MqttOptions};
use serde_json::Value;
use slog::{error, info, Logger};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::{interval, timeout, Duration};

use crate::command::Command;
use crate::command::Startup;
use crate::config::schema::{Config, Device};
use crate::homeassitant::events::RootEvent;
use crate::pages::{get_room_temperature, get_weather_and_colors};

type Client = (AsyncClient, EventLoop);

pub struct MqttC {
    pub config: Config,
    pub client: Client,
    pub running: bool,
    pub logger: Logger,
}

impl MqttC {
    pub fn new(config: Config, logger: Logger) -> Self {
        let mut mqttoptions = MqttOptions::new(
            "test-1",
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
            logger,
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
        info!(self.logger, "Entering in subscribe method");
        for device in self.config.devices.values() {
            self.client
                .0
                .subscribe(&device.mqtt.tx_topic, QoS::AtMostOnce)
                .await
                .unwrap();
            info!(
                self.logger,
                "Mqtt client is register to listen on topic {}", &device.mqtt.tx_topic
            );
        }

        let sender_to_hass = channel.0;
        let receiver_from_hass = channel.1;

        let publisher = self.client.0.clone();
        let logger = self.logger.clone();
        let config = self.config.clone();
        let shutdown_cloned = shutdown.clone();

        let hass_changes_future = async move {
            MqttC::send_on_event(
                logger,
                publisher,
                config,
                shutdown_cloned,
                receiver_from_hass,
            )
            .await;
        };
        let publisher = self.client.0.clone();
        let logger = self.logger.clone();
        let config = self.config.clone();
        let shutdown_cloned = shutdown.clone();
        let ticker_future = async move {
            MqttC::send_periodic_message(logger, publisher, config, shutdown_cloned).await;
        };

        let mqtt_handling = async move {
            while !shutdown.load(Ordering::SeqCst) {
                // MQTT event handling code goes here
                let event = timeout(Duration::from_secs(1), self.client.1.poll()).await;
                match &event {
                    Ok(Ok(e)) => {
                        match e {
                            Incoming(Publish(p)) => {
                                info!(self.logger, "Mqtt event {:?}", p);
                                let topic = std::str::from_utf8(p.topic.deref())
                                    .expect("Unable to get topic");
                                let payload = std::str::from_utf8(p.payload.deref())
                                    .expect("Unable to get payload");

                                let tx = self.commands_matching(topic, payload);
                                info!(self.logger, "TX={:?}", tx);
                                for data in tx {
                                    self.client
                                        .0
                                        .publish("tx/nspanel-ds", QoS::ExactlyOnce, false, data)
                                        .await
                                        .unwrap();
                                }
                            }
                            _ => {
                                // trace!(self.logger, "Uninteresting Mqtt event {:?}",e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!(self.logger, "Mqtt error event {:?}", e);
                    }
                    Err(_e) => {} // Timeout
                }
            }
        };
        // Execute futures concurrently
        tokio::join!(ticker_future, mqtt_handling, hass_changes_future);
    }

    async fn send_on_event(
        logger: Logger,
        publisher: AsyncClient,
        config: Config,
        shutdown: Arc<AtomicBool>,
        receiver: Arc<Mutex<Receiver<(String, String)>>>,
    ) {
        while !shutdown.load(Ordering::SeqCst) {
            // Receive messages from the shared receiver
            let message = receiver.lock().await.recv().await;
            if let Some((key, value)) = message {
                if let Some(device) = config.devices.get(key.as_str()) {
                    let messages = Self::parse_hass_event(config.clone(), device, value);
                    for message in messages {
                        publisher
                            .publish(
                                device.mqtt.rx_topic.clone(),
                                QoS::ExactlyOnce,
                                false,
                                Bytes::from(message),
                            )
                            .await
                            .unwrap();
                    }
                }
            } else {
                break; // Exit the loop if the channel is closed
            }
        }
    }

    fn parse_hass_event(config: Config, device: &Device, value: String) -> Vec<String> {
        let mut regex = String::new();
        let mut messages = vec![];
        // TemperatureSensor
        if let Some(temp_sensor) = device.get_entity_by_name(&"temperatureSensor") {
            regex.push_str(
                format!(r#"\B"{}":\{{["\+":\{{]*"s":"(.*?)"\B"#, temp_sensor.entity).as_str(),
            );
            if let Some(message) = get_room_temperature(&config, &value, temp_sensor) {
                messages.push(message);
            }
        }
        let json = serde_yaml::from_str::<RootEvent>(&*value).unwrap();
        // Weather
        if let Some(weather) = device.get_entity_by_name(&"weather") {
            if let Some(v) = json.event.entities.get(&*weather.entity) {
                // Removing cases when weather is disabled/unavailable. Unable to map to existing event struct
                if !v.to_string().contains(r#""a":{"restored":true"#) {
                    let (weather_color, weather_update) = get_weather_and_colors(&config, value, v);
                    messages.push(weather_update);
                    messages.push(weather_color);
                }
            }
        }

        messages
    }

    async fn send_periodic_message(
        logger: Logger,
        publisher: AsyncClient,
        config: Config,
        shutdown: Arc<AtomicBool>,
    ) {
        let mut interval = interval(Duration::from_secs(10)); // Create an interval of 60 seconds

        while !shutdown.load(Ordering::SeqCst) {
            // trace!(logger, "Each seconds {}", 10);
            //TODO change this to send message over channel and not like how it's done now.
            for device in config.devices.values() {
                let dt = Utc::now().with_timezone(&FixedOffset::east_opt(2 * 3600).unwrap());
                let time_str = format!("time~{:0>2}:{:0>2}~", dt.hour(), dt.minute());
                let bytes = Bytes::from(time_str.into_bytes());
                publisher
                    .publish(device.mqtt.rx_topic.clone(), QoS::ExactlyOnce, false, bytes)
                    .await
                    .unwrap();
            }
            interval.tick().await;
        }
    }
    fn commands_matching(&mut self, topic: &str, payload: &str) -> Vec<Bytes> {
        let device_id = &topic[3..topic.len()];
        info!(self.logger, "device_id {:?}", device_id);
        let config = &self.config.clone();
        let logger = self.logger.clone();
        let result = serde_json::from_str(payload)
            .map(move |data: Value| {
                let d = data.as_object();
                if let Some(o) = d {
                    let logger = self.logger.clone();
                    for (key, value) in o {
                        match key.as_str() {
                            "CustomRecv" => {
                                let tokens = value.to_string();
                                info!(logger.clone(), "Tokens {:?}", tokens);
                                if tokens.starts_with(r#""event,startup,"#) {
                                    let command: Box<dyn Command> =
                                        Box::new(Startup::new(config, device_id));
                                    return command.execute();
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
                error!(logger, "Unable to parse payload  error event {:?}", e);
            });
        result.unwrap_or_else(|_| vec![])
    }
}
