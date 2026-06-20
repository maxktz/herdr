use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::warn;

/// Theme configuration: pick a built-in or override individual tokens.
///
/// ```toml
/// [theme]
/// name = "tokyo-night"  # built-in: catppuccin, terminal, dracula, nord, etc.
///
/// [theme.custom]        # override individual tokens on top of the base
/// accent = "#f5c2e7"
/// red = "#ff6188"
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Built-in theme name. Default: "catppuccin".
    pub name: Option<String>,
    /// Follow host terminal light/dark appearance and switch between theme names.
    pub auto_switch: bool,
    /// Theme name used when `auto_switch` selects a dark appearance.
    pub dark_name: Option<String>,
    /// Theme name used when `auto_switch` selects a light appearance.
    pub light_name: Option<String>,
    /// Custom overrides — applied on top of the selected base theme.
    pub custom: Option<CustomThemeColors>,
}

/// Per-token color overrides. All fields optional — only set what you want to change.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CustomThemeColors {
    pub accent: Option<String>,
    pub panel_bg: Option<String>,
    pub surface0: Option<String>,
    pub active_space_bg: Option<String>,
    pub surface1: Option<String>,
    pub surface_dim: Option<String>,
    pub separator: Option<String>,
    pub overlay0: Option<String>,
    pub overlay1: Option<String>,
    pub text: Option<String>,
    pub subtext0: Option<String>,
    pub mauve: Option<String>,
    pub green: Option<String>,
    pub yellow: Option<String>,
    pub red: Option<String>,
    pub blue: Option<String>,
    pub teal: Option<String>,
    pub peach: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalTheme {
    pub schema_version: u32,
    pub name: String,
    pub colors: ExternalThemeColors,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalThemeColors {
    pub accent: String,
    pub panel_bg: String,
    pub surface0: String,
    pub active_space_bg: String,
    pub surface1: String,
    pub surface_dim: String,
    pub separator: String,
    pub overlay0: String,
    pub overlay1: String,
    pub text: String,
    pub subtext0: String,
    pub mauve: String,
    pub green: String,
    pub yellow: String,
    pub red: String,
    pub blue: String,
    pub teal: String,
    pub peach: String,
}

fn normalized_external_theme_name(name: &str) -> Option<String> {
    let normalized = name.trim().to_lowercase().replace([' ', '_'], "-");
    (!normalized.is_empty()
        && normalized
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'))
    .then_some(normalized)
}

fn load_external_theme_from_dir(dir: &Path, name: &str) -> Result<Option<ExternalTheme>, String> {
    let Some(normalized) = normalized_external_theme_name(name) else {
        return Err(format!("invalid external theme name: {name}"));
    };
    let path = dir.join(format!("{normalized}.toml"));
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read external theme {}: {err}", path.display()))?;
    let theme: ExternalTheme = toml::from_str(&content)
        .map_err(|err| format!("failed to parse external theme {}: {err}", path.display()))?;
    if theme.schema_version != 1 {
        return Err(format!(
            "unsupported external theme schema {} in {}; expected 1",
            theme.schema_version,
            path.display()
        ));
    }
    if normalized_external_theme_name(&theme.name).as_deref() != Some(normalized.as_str()) {
        return Err(format!(
            "external theme name {:?} does not match file {}",
            theme.name,
            path.display()
        ));
    }
    Ok(Some(theme))
}

fn external_theme_names_from_dir(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
                return None;
            }
            let name = path.file_stem()?.to_str()?;
            match load_external_theme_from_dir(dir, name) {
                Ok(Some(theme)) => Some(theme.name),
                Ok(None) => None,
                Err(err) => {
                    warn!(path = %path.display(), error = %err, "ignoring invalid external theme");
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

pub fn external_themes_dir() -> PathBuf {
    super::config_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("themes")
}

pub fn load_external_theme(name: &str) -> Result<Option<ExternalTheme>, String> {
    load_external_theme_from_dir(&external_themes_dir(), name)
}

pub fn external_theme_names() -> Vec<String> {
    external_theme_names_from_dir(&external_themes_dir())
}

/// Parse a color string into a ratatui Color.
/// Supports: hex (#rrggbb, #rgb), named colors, rgb(r,g,b), and reset aliases.
pub fn parse_color(s: &str) -> ratatui::style::Color {
    use ratatui::style::Color;
    let s = s.trim().to_lowercase();

    match s.as_str() {
        "reset" | "default" | "none" | "transparent" => return Color::Reset,
        _ => {}
    }

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::Rgb(r, g, b);
            }
        } else if hex.len() == 3 {
            let chars: Vec<u8> = hex
                .chars()
                .filter_map(|c| u8::from_str_radix(&c.to_string(), 16).ok())
                .collect();
            if chars.len() == 3 {
                return Color::Rgb(chars[0] * 17, chars[1] * 17, chars[2] * 17);
            }
        }
    }

    if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].trim().parse::<u8>(),
                parts[1].trim().parse::<u8>(),
                parts[2].trim().parse::<u8>(),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
    }

    match s.as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        _ => {
            warn!(color = s, "unknown color, defaulting to cyan");
            Color::Cyan
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn theme_name_parses() {
        let toml = r#"
[theme]
name = "dracula"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.theme.name.as_deref(), Some("dracula"));
    }

    #[test]
    fn parse_color_accepts_reset_aliases() {
        use ratatui::style::Color;

        for value in ["reset", "default", "none", "transparent"] {
            assert_eq!(parse_color(value), Color::Reset, "value: {value}");
        }
    }

    #[test]
    fn theme_auto_switch_fields_parse() {
        let toml = r#"
[theme]
name = "catppuccin"
auto_switch = true
dark_name = "tokyo-night"
light_name = "catppuccin-latte"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.theme.name.as_deref(), Some("catppuccin"));
        assert!(config.theme.auto_switch);
        assert_eq!(config.theme.dark_name.as_deref(), Some("tokyo-night"));
        assert_eq!(config.theme.light_name.as_deref(), Some("catppuccin-latte"));
    }

    #[test]
    fn theme_custom_overrides_parse() {
        let toml = r##"
[theme]
name = "nord"

[theme.custom]
panel_bg = "#1e1e2e"
separator = "#3b4261"
accent = "#ff79c6"
red = "rgb(255, 85, 85)"
"##;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.theme.name.as_deref(), Some("nord"));
        let custom = config.theme.custom.as_ref().unwrap();
        assert_eq!(custom.panel_bg.as_deref(), Some("#1e1e2e"));
        assert_eq!(custom.separator.as_deref(), Some("#3b4261"));
        assert_eq!(custom.accent.as_deref(), Some("#ff79c6"));
        assert_eq!(custom.red.as_deref(), Some("rgb(255, 85, 85)"));
        assert!(custom.green.is_none());
    }

    #[test]
    fn theme_defaults_when_missing() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.theme.name.is_none());
        assert!(!config.theme.auto_switch);
        assert!(config.theme.dark_name.is_none());
        assert!(config.theme.light_name.is_none());
        assert!(config.theme.custom.is_none());
    }

    #[test]
    fn external_theme_loads_complete_named_palette() {
        let dir = std::env::temp_dir().join(format!(
            "herdr-external-theme-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let colors = r##"
schema_version = 1
name = "ot-test"

[colors]
accent = "#000001"
panel_bg = "reset"
surface0 = "#000002"
active_space_bg = "#000003"
surface1 = "#000004"
surface_dim = "#000005"
separator = "#000006"
overlay0 = "#000007"
overlay1 = "#000008"
text = "#000009"
subtext0 = "#00000a"
mauve = "#00000b"
green = "#00000c"
yellow = "#00000d"
red = "#00000e"
blue = "#00000f"
teal = "#000010"
peach = "#000011"
"##;
        std::fs::write(dir.join("ot-test.toml"), colors).unwrap();

        let theme = load_external_theme_from_dir(&dir, "ot-test")
            .unwrap()
            .unwrap();
        assert_eq!(theme.name, "ot-test");
        assert_eq!(theme.colors.separator, "#000006");

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn external_theme_names_only_lists_valid_sorted_themes() {
        let dir = std::env::temp_dir().join(format!(
            "herdr-external-theme-list-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let colors = |name: &str| {
            format!(
                r##"schema_version = 1
name = "{name}"

[colors]
accent = "#000001"
panel_bg = "reset"
surface0 = "#000002"
active_space_bg = "#000003"
surface1 = "#000004"
surface_dim = "#000005"
separator = "#000006"
overlay0 = "#000007"
overlay1 = "#000008"
text = "#000009"
subtext0 = "#00000a"
mauve = "#00000b"
green = "#00000c"
yellow = "#00000d"
red = "#00000e"
blue = "#00000f"
teal = "#000010"
peach = "#000011"
"##
            )
        };
        std::fs::write(dir.join("ot-zeta.toml"), colors("ot-zeta")).unwrap();
        std::fs::write(dir.join("ot-alpha.toml"), colors("ot-alpha")).unwrap();
        std::fs::write(dir.join("invalid.toml"), "name = false").unwrap();
        std::fs::write(dir.join("ignored.json"), "{}").unwrap();

        assert_eq!(
            external_theme_names_from_dir(&dir),
            vec!["ot-alpha".to_string(), "ot-zeta".to_string()]
        );

        std::fs::remove_dir_all(dir).unwrap();
    }
}
