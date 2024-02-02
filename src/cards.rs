#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub(crate) enum Card{
     Screensaver,
     CardAlarm,
     CardQR,
     CardThermo,
     CardHome
}

impl Card {
     #[allow(dead_code)]
     fn as_str(&self) -> &'static str {
          match self {
               Card::Screensaver => "screensaver",
               Card::CardAlarm => "cardAlarm",
               Card::CardQR => "cardQR",
               Card::CardThermo => "cardThermo",
               Card::CardHome => "cardHome",
          }
     }
}