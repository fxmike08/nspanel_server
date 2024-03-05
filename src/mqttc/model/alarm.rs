use crate::cards::Card;
use crate::config::schema::{Config, Device, Entity};
use crate::homeassitant::events::{Alarm as AlarmD, AlarmEvent, RootEvent};
use crate::utils::{AlarmState, DeviceState};
use serde_json::Value;

pub struct Alarm {}

impl Alarm {
    /// Process the alarm data and pass back the result into the insert_message function
    /// For more details look on `Alarm::get_alarm()` function.
    pub fn process_alarm_data<F>(
        config: &Config,
        value: &str,
        device: &Device,
        json: &RootEvent,
        mut insert_message: F,
    ) where
        F: FnMut(Card, Vec<String>),
    {
        // Alarm
        if let Some(alarm) = device.get_entity_by_name(&"alarm") {
            if let Some(v) = json.event.entities.get(&*alarm.entity) {
                // Removing cases when weather is disabled/unavailable. Unable to map to existing event struct
                if !v.to_string().contains(r#""a":{"restored":true"#)
                    && !v.to_string().contains(r#"s":"unavailable"#)
                {
                    insert_message(
                        Card::CardAlarm,
                        Alarm::get_alarm(config, value, device, alarm, v),
                    );
                }
            }
        }
    }

    fn get_alarm(
        config: &Config,
        value: &str,
        device: &Device,
        alarm: Entity,
        v: &Value,
    ) -> Vec<String> {
        let mut alarm_d;
        if value.contains(format!(r#"{}":{{"s"#, alarm.entity).as_str()) {
            let a: AlarmEvent =
                serde_json::from_value(v.clone()).expect("Failed to convert to AlarmEvent struct");
            alarm_d = a;
        } else {
            let a: AlarmD =
                serde_json::from_value(v.clone()).expect("Failed to convert to Weather struct");
            alarm_d = a.event;
        }
        let mut device_state = DeviceState::default();
        let mut alarm_state = AlarmState {
            state: "".to_string(),
            supported_mode: "".to_string(),
            code_arm_required: None,
            entity: "".to_string(),
            icon: ("".to_string(), 0),
        };
        if let Some(data) = alarm_d.data {
            let mut supported_modes: Vec<&str> = vec![];
            let bits = data.supported_features.unwrap_or_default();
            if bits & 0b000001 != 0 {
                supported_modes.push("Arm Home~arm_home");
            }
            if bits & 0b000010 != 0 {
                supported_modes.push("Arm Away~arm_away");
            }
            if bits & 0b000100 != 0 {
                supported_modes.push("Arm Night~arm_night");
            }
            if bits & 0b100000 != 0 {
                supported_modes.push("Arm Vacation~arm_vacation");
            }
            alarm_state.supported_mode = supported_modes.join("~");
            alarm_state.code_arm_required = Some(data.code_arm_required.unwrap_or_default());
            alarm_state.entity = alarm.entity;
        }
        alarm_state.state = alarm_d.state.clone().unwrap_or_default();

        let mut icon: (String, u16) = ("".to_string(), 0);
        if let Some(ref state) = alarm_d.state {
            if state == "disarmed" {
                icon = (
                    config
                        .icons
                        .get("shield-off")
                        .map_or('\0', |&c| c)
                        .to_string(),
                    3334,
                );
            }
        }
        alarm_state.icon = icon;
        device_state.alarm = Some(alarm_state);

        DeviceState::read_process_overwrite(&device.id, device_state);
        let device_state = DeviceState::get_state(&device.id);
        if let Some(alarm) = device_state.alarm {
            let r_update = format!(
                "entityUpd~{}~1|1~{}~{}~{}~disable~disable~",
                alarm.entity, alarm.supported_mode, alarm.icon.0, alarm.icon.1
            )
            .into();
            return vec![r_update];
        }
        vec![]
    }
}
