use crate::cards::Card;
use crate::config::schema::{Config, Device, Entity};
use crate::homeassitant::events::RootEvent;
use serde_json::Value;

/// The Screensaver card page.
/// This is responsible for transforming data into mqtt message that can be translated by
/// Nspanel display.
pub struct Screensaver {}

impl Screensaver {
    /// Process the temperature sensor and pass back the result into the insert_message function
    /// For more details look on `Screensaver::get_room_temperature()` function.
    pub fn process_temperature_sensor<F>(
        config: &Config,
        value: &str,
        device: &Device,
        mut insert_message: F,
    ) where
        F: FnMut(Card, Vec<String>),
    {
        if let Some(temp_sensor) = device.get_entity_by_name(&"temperatureSensor") {
            insert_message(
                Card::Screensaver,
                Screensaver::get_room_temperature(config, value, temp_sensor, &device.id),
            );
        }
    }

    /// Process the weather and pass back the result into the insert_message function.
    /// For more details look on `Screensaver::get_weather_and_colors()` function.
    pub fn process_weather<F>(
        config: &Config,
        device: &Device,
        value: &str,
        json: &RootEvent,
        mut insert_message: F,
    ) where
        F: FnMut(Card, Vec<String>),
    {
        // Weather
        if let Some(weather) = device.get_entity_by_name(&"weather") {
            if let Some(v) = json.event.entities.get(&*weather.entity) {
                // Removing cases when weather is disabled/unavailable. Unable to map to existing event struct
                if !v.to_string().contains(r#""a":{"restored":true"#)
                    && !v.to_string().contains(r#"s":"unavailable"#)
                {
                    insert_message(
                        Card::Screensaver,
                        Screensaver::get_weather_and_colors(&config, value, v, weather),
                    );
                }
            }
        }
    }

    /// Extract the sensor temperature value and returning a vector that has a specific message format.
    /// * Message format
    /// ```
    /// temperature~{}~{}°C
    /// ```
    fn get_room_temperature(
        config: &Config,
        value: &str,
        temp_sensor: Entity,
        device_id: &str,
    ) -> Vec<String> {
        use crate::utils::DeviceState;
        use regex::Regex;

        let regex = format!(r#"\B"{}":\{{["\+":\{{]*"s":"(.*?)"\B"#, temp_sensor.entity);
        let rgx = Regex::new(regex.as_str()).unwrap();
        if let Ok(caps) = rgx.captures(&*value).ok_or("no match") {
            let temp = caps.get(1).map_or("", |m| m.as_str());

            let mut device_state = DeviceState::default();
            device_state.temp = Some(temp.to_string());
            DeviceState::read_process_overwrite(device_id, device_state);

            return vec![format!(
                "temperature~{}~{}°C",
                config
                    .icons
                    .get("home-thermometer-outline")
                    .map_or('\0', |&c| c),
                temp
            )];
        }
        Vec::default()
    }

    /// Extract the weather value and returning a vector that has a specific message format.
    /// * Message format for weatherUpdate,
    /// ... will repeat ~{weekday}~{color}~{tempHigh}°C~{tempLow}°C~ for each
    /// provided weather forecast ... ~
    /// ```
    /// weatherUpdate~{color}~{temp}°C~{weekday}~{color}~{tempHigh}°C~{tempLow}°C~ ... ~
    /// ```
    /// * Message format for color. For understanding each color position look
    /// at `utils.rs:DEFAULT_SCREENSAVER_COLOR_MAPPING`.
    /// ```
    /// color~0~1~2~...~21
    /// ```
    ///
    fn get_weather_and_colors(
        config: &Config,
        value: &str,
        v: &Value,
        weather_entity: Entity,
    ) -> Vec<String> {
        use crate::homeassitant::events::{Weather, WeatherEvent, WeatherForecast};
        use crate::utils::{
            get_screensaver_color_output, get_weather_icon, STORED_STATE, WEATHER_COLORS_KEY,
            WEATHER_KEY,
        };
        use chrono::{DateTime, Datelike};
        use std::collections::HashMap;

        let weather;
        if value.contains(format!(r#"{}":{{"s"#, weather_entity.entity).as_str()) {
            let w: WeatherEvent = serde_json::from_value(v.clone())
                .expect("Failed to convert to WeatherEvent struct");
            weather = w;
        } else {
            let w: Weather =
                serde_json::from_value(v.clone()).expect("Failed to convert to Weather struct");
            weather = w.event;
        }
        let mut weather_color = String::default();
        let mut weather_update = String::default();
        if weather.data.is_some() && weather.data.clone().unwrap().forecast.len() >= 4 {
            let data = weather.data.unwrap();
            // Extracting forecast_icons. Eg: Cloudy, Sunny, etc
            let forecast_icons: HashMap<String, String> = std::iter::once((
                "tMainIcon".to_string(),
                weather.state.clone().unwrap_or_default(),
            ))
            .chain(data.forecast.iter().enumerate().map(|(i, f)| {
                (
                    format!("tF{}Icon", i + 1),
                    f.condition.clone().unwrap_or_default(),
                )
            }))
            .collect();

            weather_color = get_screensaver_color_output(forecast_icons);

            let extract_weekday = |datetime_str: &str| -> chrono::Weekday {
                DateTime::parse_from_rfc3339(datetime_str)
                    .ok()
                    .map(|datetime| datetime.weekday())
                    .expect(
                        "Failed to acquire weather.data.forecast[_].datetime from weather forecast!",
                    )
            };

            let icons = &config.icons;
            let format_forecast = |forecast: &WeatherForecast| {
                format!(
                    "{}~{}~{:.1}°C~{:.1}°C",
                    extract_weekday(&forecast.datetime.clone().unwrap_or_default()),
                    get_weather_icon(forecast.condition.clone().unwrap_or_default(), icons),
                    forecast.temperature.unwrap_or(-99.9),
                    forecast.templow.unwrap_or(-99.9),
                )
            };

            let weather_icon = |condition: &str| get_weather_icon(condition.to_string(), icons);
            weather_update = format!(
                "weatherUpdate~{}~{:.1}°C~{}~{}~{}~{}",
                weather_icon(&weather.state.unwrap_or_default()),
                data.temperature.unwrap_or(-99.9),
                format_forecast(&data.forecast[0]),
                format_forecast(&data.forecast[1]),
                format_forecast(&data.forecast[2]),
                format_forecast(&data.forecast[3]),
            );
        }
        {
            let mut map = STORED_STATE
                .write()
                .expect("Failed to acquire write lock on STORED_STATE: Lock is poisoned!");
            map.insert(WEATHER_KEY.to_string(), weather_update.clone());
            map.insert(WEATHER_COLORS_KEY.to_string(), weather_color.clone());
        }
        let mut result = vec![];
        if !weather_update.is_empty() {
            result.push(weather_update);
        }
        if !weather_color.is_empty() {
            // make sure the weather_color is always after weather_update, otherwise colors will not work
            result.push(weather_color);
        }
        result
    }
}
