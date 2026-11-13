use arachnid_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisplaySettings {
    pub width: u32,
    pub height: u32,
    pub windowed: bool,
}

impl DisplaySettings {
    pub fn validate(&self) -> AppResult<()> {
        self.validate_bounds()?;
        if self.width * 9 != self.height * 16 {
            return Err(AppError::new("modDisplayUnsupported"));
        }
        Ok(())
    }

    pub fn validate_bounds(&self) -> AppResult<()> {
        if !(320..=16384).contains(&self.width) || !(200..=16384).contains(&self.height) {
            return Err(AppError::new("modDisplayUnsupported"));
        }
        Ok(())
    }
}

const TEMPLATE: &str = "[MAIN]\nFPSLimit=0\nFPSLimitMode=1\n\n[Hotkey]\nFOV_Toggle=0x39\n\n[fov]\nfov=1\n\n[Resolution]\nResolutionWidth=1920\nResolutionHeight=1080\n\n[Settings]\nWindowedMode=0\nAspectRatio=3\n";

pub fn configure(existing: Option<&[u8]>, display: &DisplaySettings) -> AppResult<Vec<u8>> {
    display.validate()?;
    let input = existing.unwrap_or(TEMPLATE.as_bytes());
    if input.len() > 64 * 1024 {
        return Err(invalid());
    }
    let input = std::str::from_utf8(input).map_err(|_| invalid())?;
    let mut section = String::new();
    let mut sections = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut result = String::new();
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].to_ascii_lowercase();
            if !sections.insert(section.clone()) {
                return Err(invalid());
            }
        }
        let Some((left, right)) = line.split_once('=') else {
            result.push_str(line);
            continue;
        };
        let key = left.trim().to_ascii_lowercase();
        let value = match (section.as_str(), key.as_str()) {
            ("resolution", "resolutionwidth") => Some(display.width),
            ("resolution", "resolutionheight") => Some(display.height),
            ("settings", "windowedmode") => Some(u32::from(display.windowed)),
            ("settings", "aspectratio") => Some(3),
            _ => None,
        };
        if let Some(value) = value {
            if !keys.insert((section.clone(), key)) {
                return Err(invalid());
            }
            let ending = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            let comment_start = [right.find("//"), right.find(';')]
                .into_iter()
                .flatten()
                .min();
            let comment = comment_start.map(|index| right[index..].trim_end_matches(['\r', '\n']));
            result.push_str(left.trim_end());
            result.push('=');
            result.push_str(&value.to_string());
            if let Some(comment) = comment {
                result.push(' ');
                result.push_str(comment);
            }
            result.push_str(ending);
        } else {
            result.push_str(line);
        }
    }
    if keys.len() != 4 {
        return Err(invalid());
    }
    Ok(result.into_bytes())
}

pub fn matches(bytes: &[u8], display: &DisplaySettings) -> bool {
    // Formatting and comments are preserved by configure; changed values produce different bytes.
    configure(Some(bytes), display)
        .is_ok_and(|configured| normalize(bytes) == normalize(&configured))
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(|line| {
            let line = line
                .split("//")
                .next()
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("");
            line.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn invalid() -> AppError {
    AppError::new("invalidFixSettings")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_display_values_preserve_unknown_keys_comments_and_line_endings() {
        let original = TEMPLATE
            .replace('\n', "\r\n")
            .replace(
                "ResolutionWidth=1920",
                "ResolutionWidth = 1920 // keep comment",
            )
            .replace("fov=1", "fov=2\r\nUnknownKey=42");
        let display = DisplaySettings {
            width: 1280,
            height: 720,
            windowed: true,
        };
        let changed = configure(Some(original.as_bytes()), &display).unwrap();
        let text = std::str::from_utf8(&changed).unwrap();
        assert!(text.contains("ResolutionWidth=1280 // keep comment\r\n"));
        assert!(text.contains("fov=2\r\nUnknownKey=42"));
        assert!(text.contains("WindowedMode=1\r\n"));
        assert!(matches(&changed, &display));
        assert!(!matches(original.as_bytes(), &display));
    }
    #[test]
    fn ambiguous_keys_unsupported_ratios_and_missing_fields_are_refused() {
        let display = DisplaySettings {
            width: 1920,
            height: 1080,
            windowed: false,
        };
        for input in [
            "[Resolution]\nResolutionWidth=1920\n",
            &TEMPLATE.replace(
                "ResolutionWidth=1920",
                "ResolutionWidth=1920\nResolutionWidth=1280",
            ),
            &TEMPLATE.replace("[Settings]", "[Settings]\n[Settings]"),
        ] {
            assert!(configure(Some(input.as_bytes()), &display).is_err());
        }
        assert!(
            configure(
                None,
                &DisplaySettings {
                    width: 3440,
                    height: 1440,
                    windowed: false
                }
            )
            .is_err()
        );
    }
}
