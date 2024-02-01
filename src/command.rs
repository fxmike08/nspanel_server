use bytes::Bytes;
use chrono::{FixedOffset, Timelike, Utc};

use crate::config::schema::Config;
use crate::utils::{STORED_STATE, WEATHER_COLORS_KEY, WEATHER_KEY};

pub struct Command<'a> {
    pub(crate) config: &'a Config,
    pub(crate) device_id: &'a str,
}

pub enum Page {
    SCREENSAVER,
    STARTUP,
}

impl<'a> Command<'_> {
    pub(crate) fn new(config: &'a Config, device_id: &'a str) -> Command<'a> {
        Command { config, device_id }
    }

    pub fn execute(&self, page: Page) -> Vec<Bytes> {
        match page {
            Page::SCREENSAVER | Page::STARTUP => self.screensaver(),
            _ => {
                vec![]
            }
        }
    }

    fn screensaver(&self) -> Vec<Bytes> {
        let dt = Utc::now().with_timezone(&FixedOffset::east_opt(2 * 3600).unwrap());
        let date = dt.format("%A, %d. %B %Y");
        let time = format!("time~{}:{}~", dt.hour(), dt.minute());

        let mut result: Vec<Bytes> = vec![
            "X".into(),
            time.into(),
            format!("date~{}", date).into(),
            format!(
                "timeout~{}",
                self.config
                    .devices
                    .get(self.device_id)
                    .expect("Failed to get device_id.")
                    .config
                    .timeout_to_screensaver
            )
            .into(),
            "dimmode~10~100~6371".into(),
            "pageType~screensaver".into(),
            "temperature~~".into(),
        ];
        {
            let map = STORED_STATE
                .read()
                .expect("Failed to acquire read lock on STORED_STATE: Lock is poisoned!");
            if let Some(weather) = map.get(WEATHER_KEY) {
                result.push(Bytes::from(weather.clone()));
            }
            if let Some(weather_colors) = map.get(WEATHER_COLORS_KEY) {
                result.push(Bytes::from(weather_colors.clone()));
            }
        }
        result
    }
}
