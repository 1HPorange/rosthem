use crate::light::LightInfo;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Bulb,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeviceInfo {
    #[serde(rename = "9001", skip_serializing_if = "Option::is_none")]
    pub label: Option<Cow<'static, str>>,
    #[serde(rename = "9003", skip_serializing_if = "Option::is_none")]
    pub id: Option<usize>,
    #[serde(rename = "3", skip_serializing_if = "Option::is_none")]
    pub product_info: Option<ProductInfo>,
    #[serde(rename = "5750", skip_serializing_if = "Option::is_none")]
    pub device_type: Option<usize>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub light_info: Option<LightInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProductInfo {
    #[serde(rename = "0", skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<Cow<'static, str>>,
    #[serde(rename = "1", skip_serializing_if = "Option::is_none")]
    pub product_name: Option<Cow<'static, str>>,
}

impl DeviceInfo {
    pub fn with_light_info(mut self, light_info: LightInfo) -> Self {
        self.light_info = Some(light_info);
        self
    }

    pub fn get_device_type(&self) -> Option<DeviceType> {
        self.device_type.map(|t| match t {
            2 => DeviceType::Bulb,
            _ => DeviceType::Unknown,
        })
    }
}
