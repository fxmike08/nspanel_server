use bytes::Bytes;
use chrono::{FixedOffset, Timelike, Utc};

use crate::config::schema::Config;

pub trait Command {
    fn execute(&self) -> Vec<Bytes>;
}

pub struct Startup<'a, 'b> {
    pub(crate) config: &'a Config,
    pub(crate) device_id: &'b str,
}

impl<'a, 'b> Startup<'a, 'b> {
    pub(crate) fn new(config: &'a Config, device_id: &'b str) -> Self {
        Startup { config, device_id }
    }
}

impl Command for Startup<'_, '_> {
    fn execute(&self) -> Vec<Bytes> {
        let dt = Utc::now().with_timezone(&FixedOffset::east_opt(2 * 3600).unwrap());
        let date = dt.format("%A, %d. %B %Y");
        let time = format!("time~{}:{}~", dt.hour(), dt.minute());
        let result: Vec<Bytes> = vec![
            "X".into(),
            time.into(),
            format!("date~{}", date).into(),
            format!(
                "timeout~{}",
                self.config
                    .devices
                    .get(self.device_id)
                    .unwrap()
                    .config
                    .timeout_to_screensaver
            )
            .into(),
            "dimmode~10~100~6371".into(),
            "pageType~screensaver".into(),
            // temp.into(),
            "temperature~~".into(),
            // r#"weatherUpdate~\xee\x96\x94~6.7\xc2\xb0C~Tue~\xee\x96\x8f~5.7\xc2\xb0C~2.3\xc2\xb0C~Wed~\xee\x96\x95~4.5\xc2\xb0C~-1.3\xc2\xb0C~Thu~\xee\x96\x8f~1.0\xc2\xb0C~-1.2\xc2\xb0C~Fri~\xee\x96\x8f~1.1\xc2\xb0C~-1.5\xc2\xb0C~~"#.into(),
            // r#"color~0~65535~65535~65535~35957~65535~65535~65535~65535~65535~31728~249~31728~31728~65535~65535~65535~65535~65535~65535~65535~65535"#.into(),
        ];
        result
    }
}
