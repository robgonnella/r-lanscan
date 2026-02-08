//! Theme and color palette definitions for the terminal UI.

use std::fmt;

use ratatui::style::{Color, palette::tailwind};

/// Color palette derived from the current theme.
#[derive(Clone, Debug)]
pub struct Colors {
    pub buffer_bg: Color,
    pub row_header_bg: Color,
    pub selected_row_fg: Color,
    pub error: Color,
    pub header_text: Color,
    pub text: Color,
    pub border_color: Color,
    pub light_gray: Color,
    pub gray: Color,
    pub input_editing: Color,
}

impl Colors {
    /// Creates a color palette from the given tailwind palette, falling back
    /// to basic colors if true color is not supported.
    pub fn new(color: &tailwind::Palette, true_color_enabled: bool) -> Self {
        let basic_colors = Self {
            buffer_bg: Color::Black,
            row_header_bg: color.c900,
            selected_row_fg: color.c400,
            error: Color::Red,
            header_text: color.c400,
            text: Color::White,
            border_color: color.c400,
            light_gray: Color::Gray,
            gray: Color::DarkGray,
            input_editing: Color::LightYellow,
        };

        let tw_colors = Self {
            buffer_bg: tailwind::SLATE.c950,
            row_header_bg: color.c900,
            selected_row_fg: color.c400,
            error: tailwind::RED.c600,
            header_text: color.c600,
            text: tailwind::SLATE.c200,
            border_color: color.c400,
            light_gray: tailwind::SLATE.c500,
            gray: tailwind::SLATE.c800,
            input_editing: tailwind::AMBER.c600,
        };

        if true_color_enabled {
            tw_colors
        } else {
            basic_colors
        }
    }
}

/// Available color themes for the application.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Theme {
    Blue,
    Emerald,
    Indigo,
    Red,
}

// Fallback palettes for terminals without true color support.
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

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Theme::Blue => write!(f, "Blue"),
            Theme::Emerald => write!(f, "Emerald"),
            Theme::Indigo => write!(f, "Indigo"),
            Theme::Red => write!(f, "Red"),
        }
    }
}

impl Theme {
    /// Parses a theme from its string name, defaulting to Blue.
    pub fn from_string(value: &str) -> Theme {
        match value {
            "Blue" => Theme::Blue,
            "Emerald" => Theme::Emerald,
            "Indigo" => Theme::Indigo,
            "Red" => Theme::Red,
            _ => Theme::Blue,
        }
    }

    /// Returns the tailwind palette for this theme, using basic colors if
    /// true color is not supported.
    pub fn to_palette(
        self,
        true_color_enabled: bool,
    ) -> &'static tailwind::Palette {
        if true_color_enabled {
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
    }
}
