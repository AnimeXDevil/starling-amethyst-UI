//! Android display choices shared by the options UI and the activity.
use crate::options_file::V;
use crate::settings::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayOption {
    Cutout,
    Orientation,
    Rotation,
    Aspect,
    SafeHorizontal,
    SafeVertical,
}

impl DisplayOption {
    pub const ALL: [Self; 6] = [
        Self::Cutout,
        Self::Orientation,
        Self::Rotation,
        Self::Aspect,
        Self::SafeHorizontal,
        Self::SafeVertical,
    ];

    pub const ASPECTS: [(i32, i32, &'static str); 11] = [
        (0, 0, "Native"),
        (1, 1, "1:1"),
        (5, 4, "5:4"),
        (4, 3, "4:3"),
        (3, 2, "3:2"),
        (16, 10, "16:10"),
        (16, 9, "16:9"),
        (18, 9, "18:9"),
        (39, 18, "19.5:9"),
        (20, 9, "20:9"),
        (21, 9, "21:9"),
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Cutout => "starlingDisplayCutout",
            Self::Orientation => "starlingDisplayOrientation",
            Self::Rotation => "starlingDisplayRotation",
            Self::Aspect => "starlingDisplayAspect",
            Self::SafeHorizontal => "starlingSafeHorizontal",
            Self::SafeVertical => "starlingSafeVertical",
        }
    }

    /// The order of the first three choices is shared with DisplayPreferences.java.
    pub const fn max(self) -> i32 {
        match self {
            Self::Cutout | Self::Orientation => 2,
            Self::Rotation => 3,
            Self::Aspect => Self::ASPECTS.len() as i32 - 1,
            Self::SafeHorizontal | Self::SafeVertical => 20,
        }
    }

    pub fn value(self, settings: &Settings) -> i32 {
        settings.file.int(self.key()).clamp(0, self.max())
    }

    pub fn set(self, settings: &mut Settings, value: i32) {
        settings
            .file
            .set(self.key(), V::Int(value.clamp(0, self.max())));
    }

    pub fn choices(self) -> &'static [&'static str] {
        match self {
            Self::Cutout => &["Protect HUD", "Avoid cutout", "Use full display"],
            Self::Orientation => &["Landscape", "Portrait", "Both"],
            Self::Rotation => &["Automatic", "Inverted automatic", "Fixed", "Fixed reversed"],
            _ => &[],
        }
    }
}

/// Insets for the HUD in whichever units its size uses. A user margin and a camera
/// inset clear the same edge, so use the larger of the two.
pub fn safe_zone(size: [f32; 2], percentages: [i32; 2], cutout: [f32; 4]) -> [f32; 4] {
    let [horizontal, vertical] =
        std::array::from_fn(|i| (size[i] * percentages[i].clamp(0, 20) as f32 / 100.0).round());
    let margins = [horizontal, vertical, horizontal, vertical];
    std::array::from_fn(|i| margins[i].max(cutout[i]).min(size[i % 2] * 0.25))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_margins_clear_cutout_and_keep_the_requested_vertical_space() {
        assert_eq!(
            safe_zone([400.0, 240.0], [10, 5], [56.0, 0.0, 0.0, 0.0]),
            [56.0, 12.0, 40.0, 12.0]
        );
    }
}
