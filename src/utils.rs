use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{debug, info};

use crate::cards::Card;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::string::ToString;
use std::sync::{Arc, RwLock};

/// Hide sensitive data from logs based on regex pattern
pub fn redact<'a>(string: &'a str, regex: &'a str) -> Cow<'a, str> {
    use regex::{Captures, Regex};

    let rgx = Regex::new(regex).expect("Failed to parse the Regex pattern");
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
    static ref DEVICE_STATE: Arc<RwLock<HashMap<String, DeviceState>>> =  Arc::new(RwLock::new(HashMap::new()));

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
#[derive(Debug, Clone)]
pub struct Page {
    pub(crate) current: Card,
    pub(crate) previous: Card,
}
impl Default for Page {
    fn default() -> Self {
        Page {
            current: Card::Screensaver,
            previous: Card::Screensaver,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlarmState {
    pub(crate) state: String,
    pub(crate) supported_mode: String,
    pub(crate) code_arm_required: Option<bool>,
    pub(crate) entity: String,
    pub(crate) icon: (String, u32), // (icon, color)
}

#[derive(Debug, Clone, Default)]
pub struct DeviceState {
    pub(crate) temp: Option<String>,
    pub(crate) humidity: Option<String>,
    pub(crate) iaq: Option<String>,
    pub(crate) page: Option<Page>,
    pub(crate) alarm: Option<AlarmState>,
}

impl DeviceState {
    // Update fields with non-None values from the provided object
    fn update_from(&mut self, other: DeviceState) {
        if let Some(temp) = other.temp {
            self.temp = Some(temp);
        }
        if let Some(humidity) = other.humidity {
            self.humidity = Some(humidity);
        }
        if let Some(iaq) = other.iaq {
            self.iaq = Some(iaq);
        }
        if let Some(page) = other.page.clone() {
            self.page = Some(page);
        }
        if let Some(alarm) = other.alarm.clone() {
            if let Some(stored) = &mut self.alarm {
                if !alarm.state.is_empty() {
                    stored.state = alarm.state;
                }
                if !alarm.supported_mode.is_empty() {
                    stored.supported_mode = alarm.supported_mode;
                }
                if alarm.code_arm_required.is_some() {
                    stored.code_arm_required = alarm.code_arm_required;
                }
                if !alarm.icon.0.is_empty() {
                    stored.icon = alarm.icon;
                }
                if !alarm.entity.is_empty() {
                    stored.entity = alarm.entity;
                }
            } else {
                self.alarm = Some(alarm);
            }
        }
    }

    // Read, model, and then overwrite the value
    pub fn read_process_overwrite(key: &str, new_state: DeviceState) {
        // Read the current value
        let current_value = {
            let read_lock = DEVICE_STATE
                .read()
                .expect("Failed to acquire read lock on DEVICE_STATE: Lock is poisoned!");
            read_lock.get(key).cloned()
        };
        let mut device_state: DeviceState;
        // Process the current value (if needed)
        if let Some(mut state) = current_value {
            debug!("Current {} device state {:?}", key, state);
            // Overwrite the value
            state.update_from(new_state);
            device_state = state;
            debug!("New {} device state {:?}", key, device_state);
        } else {
            info!(
                "Provided device_id: [{}] was not found! Creating new record.",
                key
            );
            device_state = new_state;
            device_state.page = Some(Page::default()); //Making default page
        }

        let mut write_lock = DEVICE_STATE
            .write()
            .expect("Failed to acquire write lock on DEVICE_STATE: Lock is poisoned!");
        write_lock.insert(key.to_string(), device_state);
    }

    pub fn get_state(id: &str) -> DeviceState {
        // Read the current value
        let current_value = {
            let read_lock = DEVICE_STATE
                .read()
                .expect("Failed to acquire read lock on DEVICE_STATE: Lock is poisoned!");
            read_lock.get(id).cloned()
        };
        if let Some(state) = current_value {
            state
        } else {
            DeviceState::default()
        }
    }
}
