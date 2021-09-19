use prisma::color_space::named::SRgb;
use prisma::color_space::ConvertToXyz;
use prisma::encoding::EncodableColor;
use prisma::FromColor;
use prisma::{Rgb, XyY};
use serde::{Serialize, Deserialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LightInfo {
    #[serde(rename = "3311")]
    light_options: [LightOptions; 1],
}

impl LightInfo {
    pub fn new() -> Self {
        Self {
            light_options: [LightOptions::default()],
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.light_options[0].on_off = Some(if on { 1 } else { 0 });
        self
    }

    pub fn brightness(mut self, mut brightness: u8) -> Self {
        brightness = brightness
            .saturating_sub(1)
            .saturating_add(1) // At least 1
            .saturating_add(1)
            .saturating_sub(1); // At most 255

        self.light_options[0].brightness = Some(brightness);
        self
    }

    pub fn color_preset(mut self, preset: LightColorPreset) -> Self {
        self.light_options[0].color_x = None;
        self.light_options[0].color_y = None;
        self.light_options[0].color_preset = Some(preset.to_hex().into());
        self
    }

    pub fn color_xy(mut self, x: u16, y: u16) -> Self {
        self.light_options[0].color_preset = None;
        self.light_options[0].color_x = Some(x);
        self.light_options[0].color_y = Some(y);
        self
    }

    pub fn color_rgb(mut self, rgb: &Rgb<f32>) -> Self {
        let xyz = XyY::from_color(&SRgb::new().convert_to_xyz(&rgb.srgb_encoded()));
        self.light_options[0].color_preset = None;
        self.light_options[0].color_x = Some((xyz.x() * u16::MAX as f32) as u16);
        self.light_options[0].color_y = Some((xyz.y() * u16::MAX as f32) as u16);
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
struct LightOptions {
    #[serde(rename = "5850", skip_serializing_if = "Option::is_none")]
    on_off: Option<u8>,
    #[serde(rename = "5851", skip_serializing_if = "Option::is_none")]
    brightness: Option<u8>,
    #[serde(rename = "5706", skip_serializing_if = "Option::is_none")]
    color_preset: Option<Cow<'static, str>>,
    #[serde(rename = "5709", skip_serializing_if = "Option::is_none")]
    color_x: Option<u16>,
    #[serde(rename = "5710", skip_serializing_if = "Option::is_none")]
    color_y: Option<u16>,
    //   "5712": 10 // transition time (fade time)
}

#[derive(Copy, Clone, PartialEq)]
pub enum LightColorPreset {
    Blue,
    LightBlue,
    SaturatedPurple,
    Lime,
    LightPurple,
    Yellow,
    SaturatedPink,
    DarkPeach,
    SaturatedRed,
    ColdSky,
    Pink,
    Peach,
    WarmAmber,
    LightPink,
    CoolDaylight,
    Candlelight,
    WarmGlow,
    WarmWhite,
    Sunrise,
    CoolWhite,
}

impl LightColorPreset {
    fn to_hex(&self) -> &'static str {
        match self {
            Self::Blue => "4a418a",
            Self::LightBlue => "6c83ba",
            Self::SaturatedPurple => "8f2686",
            Self::Lime => "a9d62b",
            Self::LightPurple => "c984bb",
            Self::Yellow => "d6e44b",
            Self::SaturatedPink => "d9337c",
            Self::DarkPeach => "da5d41",
            Self::SaturatedRed => "dc4b31",
            Self::ColdSky => "dcf0f8",
            Self::Pink => "e491af",
            Self::Peach => "e57345",
            Self::WarmAmber => "e78834",
            Self::LightPink => "e8bedd",
            Self::CoolDaylight => "eaf6fb",
            Self::Candlelight => "ebb63e",
            Self::WarmGlow => "efd275",
            Self::WarmWhite => "f1e0b5",
            Self::Sunrise => "f2eccf",
            Self::CoolWhite => "f5faf6",
        }
    }

    fn _from_hex(hex: &str) -> Option<Self> {
        match hex {
            "4a418a" => Some(Self::Blue),
            "6c83ba" => Some(Self::LightBlue),
            "8f2686" => Some(Self::SaturatedPurple),
            "a9d62b" => Some(Self::Lime),
            "c984bb" => Some(Self::LightPurple),
            "d6e44b" => Some(Self::Yellow),
            "d9337c" => Some(Self::SaturatedPink),
            "da5d41" => Some(Self::DarkPeach),
            "dc4b31" => Some(Self::SaturatedRed),
            "dcf0f8" => Some(Self::ColdSky),
            "e491af" => Some(Self::Pink),
            "e57345" => Some(Self::Peach),
            "e78834" => Some(Self::WarmAmber),
            "e8bedd" => Some(Self::LightPink),
            "eaf6fb" => Some(Self::CoolDaylight),
            "ebb63e" => Some(Self::Candlelight),
            "efd275" => Some(Self::WarmGlow),
            "f1e0b5" => Some(Self::WarmWhite),
            "f2eccf" => Some(Self::Sunrise),
            "f5faf6" => Some(Self::CoolWhite),
            _ => None,
        }
    }
}
