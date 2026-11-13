use crate::{fix_adapter, offline::Payload, performance};
use arachnid_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Composition {
    pub fix: bool,
    pub dxvk: bool,
    pub reshade: bool,
}

impl Composition {
    pub const DEFAULT: Self = Self {
        fix: true,
        dxvk: false,
        reshade: false,
    };

    pub fn from_bits(bits: u8) -> AppResult<Self> {
        if bits > 7 {
            return Err(AppError::new("invalidModSelection"));
        }
        Ok(Self {
            fix: bits & 1 != 0,
            dxvk: bits & 2 != 0,
            reshade: bits & 4 != 0,
        })
    }

    pub fn bits(self) -> u8 {
        u8::from(self.fix) | (u8::from(self.dxvk) << 1) | (u8::from(self.reshade) << 2)
    }

    pub fn version(self) -> String {
        format!("composition-1-{}", self.bits())
    }

    pub fn from_version(version: &str) -> AppResult<Self> {
        if ["local-preview-1", "local-preview-2", "local-preview-3"].contains(&version) {
            return Self::from_bits(7);
        }
        let bits = version
            .strip_prefix("composition-1-")
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|bits| (1..=7).contains(bits))
            .ok_or_else(|| AppError::new("invalidModState"))?;
        let result = Self::from_bits(bits)?;
        if result.version() != version {
            return Err(AppError::new("invalidModState"));
        }
        Ok(result)
    }

    pub fn binaries(self) -> Vec<Payload> {
        if self.bits() == 0 {
            return vec![];
        }
        let mut fix = performance::binaries()[0].clone();
        let mut dxvk = performance::binaries()[1].clone();
        let mut reshade = performance::vulkan_files()[0].clone();
        if self.fix {
            if !self.dxvk && !self.reshade {
                fix.sha256 = fix_adapter::ORIGINAL_SHA256.into();
            }
            let mut files = vec![fix];
            if self.dxvk {
                files.push(dxvk);
            } else if self.reshade {
                reshade.file_name = "ar_dxvk.dll".into();
                files.push(reshade);
            }
            files
        } else if self.dxvk {
            dxvk.file_name = "d3d9.dll".into();
            vec![dxvk]
        } else {
            reshade.file_name = "d3d9.dll".into();
            vec![reshade]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Composition;

    #[test]
    fn default_selection_enables_only_fix_enhancers() {
        assert_eq!(Composition::DEFAULT.bits(), 1);
    }
}
