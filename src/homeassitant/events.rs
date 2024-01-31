use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RootEvent {
    ///Subscription ID
    #[serde(alias = "id")]
    pub id: i32,
    #[serde(alias = "type")]
    pub type_: String,
    #[serde(alias = "event")]
    pub event: Event,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    #[serde(alias = "a", alias = "c")]
    pub entities: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Weather {
    #[serde(alias = "+")]
    pub event: WeatherEvent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WeatherEvent {
    #[serde(alias = "s", alias = "state")]
    pub state: String,
    #[serde(alias = "lc")]
    pub last_changed: f64, // Unix epoch time
    #[serde(alias = "c")]
    pub context: String,

    #[serde(alias = "a")]
    pub data: WeatherEventData,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WeatherEventData {
    pub temperature: Option<f32>,
    pub apparent_temperature: Option<f32>,
    pub dew_point: Option<f32>,
    pub temperature_unit: Option<String>,
    pub humidity: Option<u8>,
    pub cloud_coverage: Option<u8>,
    pub uv_index: Option<u8>,
    pub pressure: Option<f32>,
    pub pressure_unit: Option<String>,
    pub wind_bearing: Option<u16>,
    pub wind_gust_speed: Option<f32>,
    pub wind_speed: Option<f32>,
    pub wind_speed_unit: Option<String>,
    pub visibility: Option<f32>,
    pub visibility_unit: Option<String>,
    pub precipitation_unit: Option<String>,
    pub forecast: Vec<WeatherForecast>,
    pub friendly_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WeatherForecast {
    pub datetime: Option<String>,
    pub cloud_coverage: Option<u8>,
    pub precipitation_probability: Option<u8>,
    pub uv_index: Option<u8>,
    pub wind_bearing: Option<u16>,
    pub condition: Option<String>,
    pub temperature: Option<f32,>,
    pub apparent_temperature: Option<f32>,
    pub templow: Option<f32>,
    pub wind_gust_speed: Option<f32>,
    pub wind_speed: Option<f32>,
    pub precipitation: Option<f32>,
}
