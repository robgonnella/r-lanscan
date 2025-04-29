use ratatui::style::{palette::tailwind, Color};

#[derive(Clone, Debug)]
pub struct Colors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub selected_row_fg: Color,
    pub row_fg: Color,
    pub row_bg: Color,
    pub border_color: Color,
    pub scroll_bar_fg: Color,
    pub label: Color,
    pub input_editing: Color,
}

impl Colors {
    pub fn new(color: &tailwind::Palette) -> Self {
        let basic_colors = Self {
            buffer_bg: Color::Black,
            header_bg: color.c900,
            header_fg: Color::Black,
            selected_row_fg: color.c400,
            row_fg: Color::White,
            row_bg: Color::Black,
            border_color: color.c400,
            scroll_bar_fg: Color::Black,
            label: color.c400,
            input_editing: Color::LightYellow,
        };

        let tw_colors = Self {
            buffer_bg: Color::Black,
            header_bg: color.c900,
            header_fg: Color::Black,
            selected_row_fg: color.c400,
            row_fg: Color::White,
            row_bg: Color::Black,
            border_color: color.c400,
            scroll_bar_fg: Color::Black,
            label: color.c400,
            input_editing: tailwind::AMBER.c600,
        };

        if let Some(support) = supports_color::on(supports_color::Stream::Stdout) {
            if support.has_16m {
                tw_colors
            } else {
                basic_colors
            }
        } else {
            basic_colors
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Theme {
    Blue,
    Emerald,
    Indigo,
    Red,
}

const BASIC_BLUE_PALLETE: tailwind::Palette = tailwind::Palette {
    c50: Color::LightCyan,
    c100: Color::LightCyan,
    c200: Color::LightCyan,
    c300: Color::LightCyan,
    c400: Color::LightCyan,
    c500: Color::Cyan,
    c600: Color::Cyan,
    c700: Color::Cyan,
    c800: Color::Cyan,
    c900: Color::Cyan,
    c950: Color::Cyan,
};

const BASIC_RED_PALLETE: tailwind::Palette = tailwind::Palette {
    c50: Color::LightRed,
    c100: Color::LightRed,
    c200: Color::LightRed,
    c300: Color::LightRed,
    c400: Color::LightRed,
    c500: Color::Red,
    c600: Color::Red,
    c700: Color::Red,
    c800: Color::Red,
    c900: Color::Red,
    c950: Color::Red,
};

const BASIC_GREEN_PALLETE: tailwind::Palette = tailwind::Palette {
    c50: Color::LightGreen,
    c100: Color::LightGreen,
    c200: Color::LightGreen,
    c300: Color::LightGreen,
    c400: Color::LightGreen,
    c500: Color::Green,
    c600: Color::Green,
    c700: Color::Green,
    c800: Color::Green,
    c900: Color::Green,
    c950: Color::Green,
};

const BASIC_MAGENTA_PALLETE: tailwind::Palette = tailwind::Palette {
    c50: Color::LightMagenta,
    c100: Color::LightMagenta,
    c200: Color::LightMagenta,
    c300: Color::LightMagenta,
    c400: Color::LightMagenta,
    c500: Color::Magenta,
    c600: Color::Magenta,
    c700: Color::Magenta,
    c800: Color::Magenta,
    c900: Color::Magenta,
    c950: Color::Magenta,
};

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
        if let Some(support) = supports_color::on(supports_color::Stream::Stdout) {
            if support.has_16m {
                match self {
                    Theme::Blue => &tailwind::BLUE,
                    Theme::Emerald => &tailwind::EMERALD,
                    Theme::Indigo => &tailwind::INDIGO,
                    Theme::Red => &tailwind::RED,
                }
            } else {
                match self {
                    Theme::Blue => &BASIC_BLUE_PALLETE,
                    Theme::Red => &BASIC_RED_PALLETE,
                    Theme::Indigo => &BASIC_MAGENTA_PALLETE,
                    Theme::Emerald => &BASIC_GREEN_PALLETE,
                }
            }
        } else {
            match self {
                Theme::Blue => &BASIC_BLUE_PALLETE,
                Theme::Red => &BASIC_RED_PALLETE,
                Theme::Indigo => &BASIC_MAGENTA_PALLETE,
                Theme::Emerald => &BASIC_GREEN_PALLETE,
            }
        }
    }
}
