mod libcoap;

use angular_units::Deg;
pub use libcoap::{
    error::CoapError,
    light::{LightColorPreset, LightInfo},
    *,
};
use prisma::FromColor;
use prisma::{Hsv, Rgb};
use std::net::Ipv4Addr;
use std::thread::sleep;
use std::time::Duration;
