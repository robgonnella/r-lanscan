use ratatui::style::palette::tailwind;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ViewName {
    Device,
    Devices,
    Config,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Theme {
    Blue,
    Emerald,
    Indigo,
    Red,
}

impl Theme {
    pub fn from_string(value: &String) -> Theme {
        match value.as_str() {
            "Blue" => Theme::Blue,
            "Emerald" => Theme::Emerald,
            "Indigo" => Theme::Indigo,
            "Red" => Theme::Red,
            _ => Theme::Blue,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Theme::Blue => "Blue".to_string(),
            Theme::Emerald => "Emerald".to_string(),
            Theme::Indigo => "Indigo".to_string(),
            Theme::Red => "Red".to_string(),
        }
    }

    pub fn to_palette(&self) -> &'static tailwind::Palette {
        match self {
            Theme::Blue => &tailwind::BLUE,
            Theme::Emerald => &tailwind::EMERALD,
            Theme::Indigo => &tailwind::INDIGO,
            Theme::Red => &tailwind::RED,
        }
    }
}
