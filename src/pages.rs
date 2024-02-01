use crate::config::schema::{Config, Entity};
use crate::homeassitant::events::{Weather, WeatherEvent, WeatherForecast};
use crate::utils::{
    get_screensaver_color_output, get_weather_icon, STORED_STATE, WEATHER_COLORS_KEY, WEATHER_KEY,
};
use std::collections::HashMap;

use serde_json::Value;

pub fn get_weather_and_colors(config: &Config, value: String, v: &Value) -> (String, String) {
    use chrono::{DateTime, Datelike};

    let weather;
    if value.contains(r#"weather.accuweather":{"s"#) {
        let w: WeatherEvent =
            serde_json::from_value(v.clone()).expect("Failed to convert to WeatherEvent struct");
        weather = w;
    } else {
        let w: Weather =
            serde_json::from_value(v.clone()).expect("Failed to convert to Weather struct");
        weather = w.event;
    }
    let mut weather_color = String::default();
    let mut weather_update = String::default();
    if weather.data.forecast.len() >= 4 {
        // Extracting forecast_icons. Eg: Cloudy, Sunny, etc
        let forecast_icons: HashMap<String, String> = std::iter::once((
            "tMainIcon".to_string(),
            weather.state.clone().unwrap_or_default(),
        ))
        .chain(weather.data.forecast.iter().enumerate().map(|(i, f)| {
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
            weather.data.temperature.unwrap_or(-99.9),
            format_forecast(&weather.data.forecast[0]),
            format_forecast(&weather.data.forecast[1]),
            format_forecast(&weather.data.forecast[2]),
            format_forecast(&weather.data.forecast[3]),
        );
    }
    {
        let mut map = STORED_STATE
            .write()
            .expect("Failed to acquire write lock on STORED_STATE: Lock is poisoned!");
        map.insert(WEATHER_KEY.to_string(), weather_update.clone());
        map.insert(WEATHER_COLORS_KEY.to_string(), weather_color.clone());
    }
    (weather_color, weather_update)
}

pub fn get_room_temperature(
    config: &Config,
    value: &String,
    temp_sensor: Entity,
) -> Option<String> {
    use regex::Regex;

    let regex = format!(r#"\B"{}":\{{["\+":\{{]*"s":"(.*?)"\B"#, temp_sensor.entity);
    let rgx = Regex::new(regex.as_str()).unwrap();
    if let Ok(caps) = rgx.captures(&*value).ok_or("no match") {
        return Some(format!(
            "temperature~{}~{}°C",
            config.icons.get("home-thermometer-outline").map_or('\0', |&c| c),
            caps.get(1).map_or("", |m| m.as_str())
        ));
    }
    None
}
