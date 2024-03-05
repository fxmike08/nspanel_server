use crate::cards::Card;
use bytes::Bytes;
use chrono::{FixedOffset, Timelike, Utc};

use crate::config::schema::Config;
use crate::utils::{DeviceState, STORED_STATE, WEATHER_COLORS_KEY, WEATHER_KEY};

pub struct Command<'a> {
    pub(crate) config: &'a Config,
    pub(crate) device_id: &'a str,
}

pub enum Page {
    Screensaver,
    Startup,
    ExistScreensaver,
    CardAlarm,
    CardQR,
}
impl From<&str> for Page {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "screensaver" => Self::Screensaver,
            "startup" => Self::Startup,
            "existscreensaver" => Self::ExistScreensaver,
            "cardalarm" => Self::CardAlarm,
            "cardqr" => Self::CardQR,
            _ => panic!(
                "Invalid string representation for Page::{} enum variant",
                value
            ),
        }
    }
}

impl<'a> Command<'_> {
    pub(crate) fn new(config: &'a Config, device_id: &'a str) -> Command<'a> {
        Command { config, device_id }
    }

    pub fn execute(&self, page: Page) -> Vec<Bytes> {
        match page {
            Page::Screensaver | Page::Startup => self.screensaver(),
            Page::ExistScreensaver => self.exist_screensaver(),
            Page::CardAlarm => self.card_alarm(),
            Page::CardQR => self.qr_code(),
            _ => {
                vec![]
            }
        }
    }

    fn exist_screensaver(&self) -> Vec<Bytes> {
        let mut device = DeviceState::get_state(self.device_id);
        if let Some(mut page) = device.page.take() {
            if page.current == page.previous && page.current == Card::Screensaver {
                if let Some(first_card) = self
                    .config
                    .devices
                    .get(self.device_id)
                    .expect("Failed to get device_id.")
                    .get_cards()
                    .get(0)
                    .map(|card| card.type_.clone())
                {
                    page.current = Card::from(first_card);
                }
            }
            device.page = Some(page);
        }
        DeviceState::read_process_overwrite(self.device_id, device);

        self.qr_code() // TODO FIX ME
                       // vec![card]
    }

    fn card_alarm(&self) -> Vec<Bytes> {
        let mut r_page = Bytes::default();
        let mut r_update = Bytes::default();
        let mut device = DeviceState::get_state(self.device_id);
        if let Some(mut page) = device.page.take() {
            page.previous = page.current;
            page.current = Card::CardAlarm;
        }

        r_page = format!("pageType~{}", Card::CardAlarm.as_str()).into();
        if let Some(alarm) = &device.alarm {
            r_update = format!(
                "entityUpd~{}~1|1~{}~{}~{}~disable~disable~",
                alarm.entity, alarm.supported_mode, alarm.icon.0, alarm.icon.1
            )
            .into();
        }
        DeviceState::read_process_overwrite(self.device_id, device);

        vec![r_page, r_update]
    }
    fn screensaver(&self) -> Vec<Bytes> {
        let dt = Utc::now().with_timezone(&FixedOffset::east_opt(2 * 3600).unwrap());
        let date = dt.format("%A, %d. %B %Y");
        let time = format!("time~{:0>2}:{:0>2}", dt.hour(), dt.minute());
        let temp = DeviceState::get_state(self.device_id)
            .temp
            .unwrap_or_default();
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
            format!(
                "temperature~{}~{}°C",
                self.config
                    .icons
                    .get("home-thermometer-outline")
                    .map_or('\0', |&c| c),
                temp
            )
            .into(),
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

    fn qr_code(&self) -> Vec<Bytes> {
        let device_state = DeviceState::get_state(self.device_id);

        let mut r_page = Bytes::default();
        let mut r_update = Bytes::default();
        if let Some(page) = device_state.page {
            r_page = format!("pageType~{}", page.current.as_str()).into();

            if let Some(config_card) = self
                .config
                .get_card_by_name(self.device_id, page.current.as_str())
            {
                // 0|0 means it's only one element
                // 1|1 means we have multiple cards
                // 2|0 is like Up button
                r_update = format!(
                    "entityUpd~{}~1|1~{}~text~{}~{}~{}~Name~{}~text~{}~{}~{}~Password~{}",
                    config_card.title.unwrap_or_default(),
                    config_card.data.unwrap_or_default(),
                    config_card.entities[0].entity,
                    self.config
                        .icons
                        .get(&config_card.entities[0].icon.clone().unwrap_or_default())
                        .map_or('\0', |&c| c), // Icon
                    17299, //Color
                    config_card.entities[0].name.clone().unwrap_or_default(),
                    config_card.entities[1].entity,
                    self.config
                        .icons
                        .get(&config_card.entities[1].icon.clone().unwrap_or_default())
                        .map_or('\0', |&c| c), // Icon
                    17299, //Color
                    config_card.entities[1].name.clone().unwrap_or_default()
                )
                .into();
            }
        }

        vec![r_page, r_update]
    }
}
