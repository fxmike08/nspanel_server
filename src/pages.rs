use crate::config::schema::{Config, Entity};
use crate::homeassitant::events::{Weather, WeatherEvent};
use crate::utils::{
    get_screensaver_color_output, get_weather_icon, STORED_STATE, WEATHER_COLORS_KEY, WEATHER_KEY,
};
use chrono::{DateTime, Datelike, FixedOffset};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

pub fn get_weather_and_colors(config: &Config, value: String, v: &Value) -> (String, String) {
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
    let mut weather_color = "".to_string();
    let mut weather_update = "".to_string();
    if weather.data.forecast.len() >= 4 {
        weather_color = get_screensaver_color_output(HashMap::from([
            ("tMainIcon".to_string(), weather.state.clone()),
            (
                "tF1Icon".to_string(),
                weather.data.forecast[0].condition.clone(),
            ),
            (
                "tF2Icon".to_string(),
                weather.data.forecast[1].condition.clone(),
            ),
            (
                "tF3Icon".to_string(),
                weather.data.forecast[2].condition.clone(),
            ),
            (
                "tF4Icon".to_string(),
                weather.data.forecast[3].condition.clone(),
            ),
        ]));
        let mut datetime: DateTime<FixedOffset> =
            DateTime::parse_from_rfc3339(weather.data.forecast[0].datetime.as_str()).unwrap();
        let weekday_now = datetime.weekday();

        datetime =
            DateTime::parse_from_rfc3339(weather.data.forecast[1].datetime.as_str()).unwrap();
        let tomorrow = datetime.weekday();

        datetime =
            DateTime::parse_from_rfc3339(weather.data.forecast[2].datetime.as_str()).unwrap();
        let day_after_tomorrow = datetime.weekday();
        datetime =
            DateTime::parse_from_rfc3339(weather.data.forecast[3].datetime.as_str()).unwrap();
        let day_after_after_tomorrow = datetime.weekday();

        weather_update = format!(
            "weatherUpdate~{}~{:.1}°C~{}~{}~{:.1}°C~{:.1}°C~{}~{}~{:.1}°C~{:.1}°C~{}~{}~{:.1}°C~{:.1}°C~{}~{}~{:.1}°C~{:.1}°C",
            get_weather_icon(weather.state, &config.icons),
            weather.data.temperature,
            weekday_now,
            get_weather_icon(
                weather.data.forecast[0].condition.clone(),
                &config.icons
            ),
            weather.data.forecast[0].temperature,
            weather.data.forecast[0].templow,
            tomorrow,
            get_weather_icon(
                weather.data.forecast[1].condition.clone(),
                &config.icons
            ),
            weather.data.forecast[1].temperature,
            weather.data.forecast[1].templow,
            day_after_tomorrow,
            get_weather_icon(
                weather.data.forecast[2].condition.clone(),
                &config.icons
            ),
            weather.data.forecast[2].temperature,
            weather.data.forecast[2].templow,
            day_after_after_tomorrow,
            get_weather_icon(
                weather.data.forecast[3].condition.clone(),
                &config.icons
            ),
            weather.data.forecast[3].temperature,
            weather.data.forecast[3].templow,
        );
    }
    {
        let mut map = STORED_STATE.write().expect("Poisoned lock");
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
    let regex = format!(r#"\B"{}":\{{["\+":\{{]*"s":"(.*?)"\B"#, temp_sensor.entity);
    let rgx = Regex::new(regex.as_str()).unwrap();
    if let Ok(caps) = rgx.captures(&*value).ok_or("no match") {
        return Some(format!(
            "temperature~{}~{}°C",
            config.icons.get("home-thermometer-outline").unwrap(),
            caps.get(1).map_or("", |m| m.as_str())
        ));
    }
    None
}
