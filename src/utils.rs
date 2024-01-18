use indexmap::IndexMap;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::string::ToString;
use std::sync::{Arc, RwLock};

/// Hide sensitive data from logs based on regex pattern
pub fn redact<'a>(string: &'a str, regex: &'a str) -> Cow<'a, str> {
    let rgx = Regex::new(regex).unwrap();
    let res = rgx.replace_all(string, |caps: &Captures| {
        if caps.get(1).is_some() {
            let g0 = caps.get(0).unwrap();
            let g1 = caps.get(1).unwrap();
            return Cow::from(g0.as_str().replace(g1.as_str(), "****"));
        } else {
            let g0 = caps.get(0).unwrap();
            let m = caps.get(2).unwrap();
            return Cow::from(g0.as_str().replace(m.as_str(), "****"));
        }
    });
    res
}

pub const WEATHER_KEY: &str = "weather";
pub const WEATHER_COLORS_KEY: &str = "weather_colors";

lazy_static! {
    pub static ref STORED_STATE: Arc<RwLock<HashMap<String, String>>> =  Arc::new(RwLock::new(HashMap::new()));

    pub static ref WEATHER_COLORS: HashMap<String, u32> =
        HashMap::from([
        //#50% grey
        ("partlycloudy".into(), 35957),
        ("windy".into(), 35957),
        // #yellow grey
        ("clear-night".into(), 35957),
        // #red grey
        ("windy-variant".into(), 35957),
        // #grey-blue
        ("cloudy".into(), 31728),
        // #red
        ("exceptional".into(), 63488),
        // #75% grey
        ("fog".into(), 21130),
        //  #white
        ("hail".into(), 65535),
        ("snowy".into(), 65535),
        // #golden-yellow
        ("lightning".into(), 65120),
        // #dark-golden-yellow
        ("lightning-rainy".into(), 50400),
        // #blue
        ("pouring".into(), 249 ),
        // #light-blue
        ("rainy".into(), 33759),
        // #light-blue-grey
        ("snowy-rainy".into(), 44479),
        // #bright-yellow
        ("sunny".into(), 63469),
    ]);
    pub static ref WEATHER_MAPPING: HashMap<String, String> =
    HashMap::from([
            ("clear-night".into(), "weather-night".into()),
            ("cloudy".into(), "weather-cloudy".into()),
            ("exceptional".into(), "alert-circle-outline".into()),
            ("fog".into(), "weather-fog".into()),
            ("hail".into(), "weather-hail".into()),
            ("lightning-rainy".into(), "weather-lightning-rainy".into()),
            ("partlycloudy".into(), "weather-partly-cloudy".into()),
            ("pouring".into(), "weather-pouring".into()),
            ("rainy".into(), "weather-rainy".into()),
            ("snowy".into(), "weather-snowy".into()),
            ("snowy-rainy".into(), "weather-snowy-rainy".into()),
            ("sunny".into(), "weather-sunny".into()),
            ("windy".into(), "weather-windy".into()),
            ("windy-variant".into(), "weather-windy-variant".into()),
        ]);


    pub static ref DEFAULT_SCREENSAVER_COLOR_MAPPING: IndexMap<String, u32> =
    IndexMap::from([
        ("background".into(),       0),
        ("time".into(),             65535),
        ("timeAMPM".into(),         65535),
        ("date".into(),             65535),
        ("tMainIcon".into(),        65535),
        ("tMainText".into(),        65535),
        ("tForecast1".into(),       65535),
        ("tForecast2".into(),       65535),
        ("tForecast3".into(),       65535),
        ("tForecast4".into(),       65535),
        ("tF1Icon".into(),          65535),
        ("tF2Icon".into(),          65535),
        ("tF3Icon".into(),          65535),
        ("tF4Icon".into(),          65535),
        ("tForecast1Val".into(),    65535),
        ("tForecast2Val".into(),    65535),
        ("tForecast3Val".into(),    65535),
        ("tForecast4Val".into(),    65535),
        ("bar".into(),              65535),
        ("tMRIcon".into(),          65535),
        ("tMR".into(),              65535),
        ("tTimeAdd".into(),         65535),
    ]);
}

pub fn get_weather_icon(state: String, icons: &BTreeMap<String, char>) -> char {
    if let Some(e) = WEATHER_MAPPING.get(&state) {
        if let Some(icon) = icons.get(e) {
            return *icon;
        }
    }
    return '\0';
}

pub fn get_screensaver_color_output(icons: HashMap<String, String>) -> String {
    let keys = [
        "tMainIcon".to_string(),
        "tF1Icon".to_string(),
        "tF2Icon".to_string(),
        "tF3Icon".to_string(),
        "tF4Icon".to_string(),
    ];
    let mut color_output = "color".to_string();
    for (key, value) in DEFAULT_SCREENSAVER_COLOR_MAPPING.iter() {
        if keys.contains(key) {
            if let Some(weather) = icons.get(key) {
                if let Some(color) = WEATHER_COLORS.get(weather) {
                    color_output += &*format!("~{}", color);
                } else {
                    color_output += &*format!("~{}", value);
                }
            } else {
                color_output += &*format!("~{}", value);
            }
        } else {
            color_output += &*format!("~{}", value);
        }
    }
    color_output
}
