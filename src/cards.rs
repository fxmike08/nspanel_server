#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub(crate) enum Card {
    Screensaver,
    CardQR,
    CardAlarm,
    CardThermo,
    CardHome,
}

impl From<String> for Card {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "screensaver" => Card::Screensaver,
            "cardqr" => Card::CardQR,
            "cardalarm" => Card::CardAlarm,
            "cardthermo" => Card::CardThermo,
            "cardhome" => Card::CardHome,
            _ => panic!("Invalid string representation for Card enum variant"),
        }
    }
}
impl Card {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Card::Screensaver => "screensaver",
            Card::CardQR => "cardQR",
            Card::CardAlarm => "cardAlarm",
            Card::CardThermo => "cardThermo",
            Card::CardHome => "cardHome",
        }
    }
}
