use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub module: String,
    pub id: String,
    pub mqtt: Mqtt,
    pub model: Model,
    pub config: DeviceConfig,
    pub cards: Vec<Cards>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Model {
    EU,
    US,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mqtt {
    pub rx_topic: String,
    pub tx_topic: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceConfig {
    pub timeout_to_screensaver: u16,
    pub screensaver_brightness: Vec<BrightnessScheduler>,
    pub locale: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrightnessScheduler {
    pub time: String,
    pub value: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cards {
    #[serde(alias = "type")]
    pub type_: String,
    pub title: Option<String>,
    pub data: Option<String>,
    pub entities: Vec<Entity>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub entity: String,
    pub name: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MqttClient {
    #[serde(alias = "type")]
    pub type_: String,
    #[serde(alias = "client_id")]
    pub id: String,
    #[serde(alias = "client_host")]
    pub host: String,
    #[serde(alias = "client_port")]
    pub port: u16,
    #[serde(alias = "client_user")]
    pub user: String,
    #[serde(alias = "client_password")]
    pub password: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hass {
    #[serde(alias = "type")]
    pub type_: String,
    pub host: String,
    pub port: u16,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connectivity {
    #[serde(alias = "MQTT", alias = "mqttc")]
    pub mqtt: MqttClient,
    #[serde(alias = "hass")]
    pub hass: Hass,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub(crate) connectivity: Connectivity,
    pub(crate) devices: BTreeMap<String, Device>,
    pub(crate) icons: BTreeMap<String, char>,
}

impl Config {
    pub fn get_entities(&self) -> BTreeMap<String, Vec<String>> {
        self.devices
            .clone()
            .into_iter()
            .map(|(key, device)| {
                let res: Vec<String> = device
                    .cards
                    .iter()
                    .flat_map(|c| c.entities.iter().map(|e| e.entity.clone()))
                    .collect();
                (key, res)
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_entity_by_name(&self, device_id: &str, name: &str) -> Option<Entity> {
        self.devices
            .clone()
            .into_iter()
            .find(|(key, _)| key.eq(device_id))
            .and_then(|(_, device)| {
                device.cards.into_iter().find_map(|c| {
                    c.entities
                        .into_iter()
                        .find(|e| e.name.as_deref() == Some(name))
                })
            })
    }
}

impl Device {
    pub fn get_entity_by_name(&self, name: &str) -> Option<Entity> {
        self.cards.clone().into_iter().find_map(|c| {
            c.entities
                .into_iter()
                .find(|e| e.name.as_deref() == Some(name))
        })
    }
}
