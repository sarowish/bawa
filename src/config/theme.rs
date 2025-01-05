use anyhow::{Context, Result};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

#[derive(Deserialize)]
struct UserStyle {
    fg: Option<String>,
    bg: Option<String>,
    modifiers: Option<String>,
}

impl UserStyle {
    fn to_style(&self) -> Result<Style> {
        let mut style = Style::default();

        if let Some(fg) = &self.fg {
            style = style.fg(str_to_color(fg)?);
        }

        if let Some(bg) = &self.bg {
            style = style.bg(str_to_color(bg)?);
        }

        if let Some(modifiers) = &self.modifiers {
            style.add_modifier = parse_modifiers(modifiers)?;
        }

        Ok(style)
    }
}

fn parse_hex(color: &str) -> Result<Color> {
    if color.len() != 7 {
        anyhow::bail!("Hex string must be 7 characters long.")
    }

    let red = u8::from_str_radix(&color[1..3], 16)?;
    let green = u8::from_str_radix(&color[3..5], 16)?;
    let blue = u8::from_str_radix(&color[5..7], 16)?;

    Ok(Color::Rgb(red, green, blue))
}

fn parse_rgb(color: &str) -> Result<Color> {
    let primaries: Vec<&str> = color.split(',').map(str::trim).collect();

    if primaries.len() != 3 {
        anyhow::bail!("RGB string must be composed of three primary colors.");
    }

    let red = primaries[0].parse()?;
    let green = primaries[1].parse()?;
    let blue = primaries[2].parse()?;

    Ok(Color::Rgb(red, green, blue))
}

fn str_to_color(color: &str) -> Result<Color> {
    let color = match color {
        "Black" => Color::Black,
        "Red" => Color::Red,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Blue" => Color::Blue,
        "Magenta" => Color::Magenta,
        "Cyan" => Color::Cyan,
        "Gray" => Color::Gray,
        "DarkGray" => Color::DarkGray,
        "LightRed" => Color::LightRed,
        "LightGreen" => Color::LightGreen,
        "LightYellow" => Color::LightYellow,
        "LightBlue" => Color::LightBlue,
        "LightMagenta" => Color::LightMagenta,
        "LightCyan" => Color::LightCyan,
        "White" => Color::White,
        "Reset" => Color::Reset,
        color if color.starts_with('#') => parse_hex(color).with_context(|| {
            format!(
                "\"{color}\" is an invalid hex color. \
                    It must be of the form \"#xxxxxx\" where every x is a hexadecimal digit."
            )
        })?,
        color if color.contains(',') => parse_rgb(color).with_context(|| {
            format!(
                "\"{color}\" is an invalid RGB color. \
                    It must be of the form \"x, x, x\" where every x is an integer from 0 to 255."
            )
        })?,
        _ => anyhow::bail!(
            "\"{}\" is not a valid color name. \
            Valid color names are Black, Red, Green, Yellow, Blue, Magenta \
            Cyan, Gray, DarkGray, LightRed, LightGreen, LightGreen, \
            LightYellow, LightBlue, LightMagenta, LightCyan, White and Reset.",
            color
        ),
    };

    Ok(color)
}

fn parse_modifiers(modifiers: &str) -> Result<Modifier> {
    let mut res = Modifier::empty();

    for modifier in modifiers.split_whitespace() {
        res.insert(match modifier {
            "bold" => Modifier::BOLD,
            "dim" => Modifier::DIM,
            "italic" => Modifier::ITALIC,
            "underlined" => Modifier::UNDERLINED,
            "slow_blink" => Modifier::SLOW_BLINK,
            "rapid_blink" => Modifier::RAPID_BLINK,
            "reversed" => Modifier::REVERSED,
            "hidden" => Modifier::HIDDEN,
            "crossed_out" => Modifier::CROSSED_OUT,
            _ => anyhow::bail!(
                "\"{}\" is not a valid modifier. \
                Valid modifiers are bold, dim, italic, underlined, \
                slow_blink, rapid_blink, reversed, hidden and crossed_out.",
                modifier
            ),
        });
    }

    Ok(res)
}

#[derive(Deserialize)]
pub struct UserTheme {
    title: Option<UserStyle>,
    selected: Option<UserStyle>,
    marked: Option<UserStyle>,
    active: Option<UserStyle>,
    fuzzy_selected: Option<UserStyle>,
    highlight: Option<UserStyle>,
    confirmation_border: Option<UserStyle>,
    error: Option<UserStyle>,
    warning: Option<UserStyle>,
    help: Option<UserStyle>,
}

pub struct Theme {
    pub title: Style,
    pub selected: Style,
    pub marked: Style,
    pub active: Style,
    pub fuzzy_selected: Style,
    pub highlight: Style,
    pub confirmation_border: Style,
    pub error: Style,
    pub warning: Style,
    pub help: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            title: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            selected: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            marked: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::CROSSED_OUT),
            active: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            fuzzy_selected: Style::default().fg(Color::Magenta),
            highlight: Style::default().fg(Color::Yellow),
            confirmation_border: Style::default().fg(Color::Blue),
            error: Style::default().fg(Color::Red),
            warning: Style::default().fg(Color::Yellow),
            help: Style::default().fg(Color::Green),
        }
    }
}

impl TryFrom<UserTheme> for Theme {
    type Error = anyhow::Error;

    fn try_from(user_theme: UserTheme) -> Result<Self, Self::Error> {
        let mut theme = Theme::default();

        macro_rules! set_theme_field {
            ($name: ident) => {
                if let Some(color) = user_theme.$name {
                    theme.$name = UserStyle::to_style(&color).with_context(|| {
                        format!("Error: couldn't set a field of \"{}\"", stringify!($name))
                    })?;
                }
            };
        }

        set_theme_field!(title);
        set_theme_field!(selected);
        set_theme_field!(marked);
        set_theme_field!(active);
        set_theme_field!(fuzzy_selected);
        set_theme_field!(highlight);
        set_theme_field!(confirmation_border);
        set_theme_field!(error);
        set_theme_field!(warning);
        set_theme_field!(help);

        Ok(theme)
    }
}
