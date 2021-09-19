#![allow(unused_imports)]

use std::{thread::sleep, time::Duration};

use angular_units::Deg;
use prisma::{FromColor, Hsv, Rgb};
use rosthem::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    uri: String,
    ip: String,
    user: String,
    key: String,
}

fn main() -> Result<(), CoapError> {
    const IO_FREQUENCY: Option<Duration> = Some(Duration::from_millis(100));

    let config: Config =
        serde_json::from_str(&std::fs::read_to_string("./config.json").expect("Missing config"))
            .expect("Invalid config");

    let coap = Coap::new(Some(CoapLogLevel::Debug))?;
    let context = coap.new_context()?;

    let uri = CoapUri::new(config.uri)?;
    let optlist = CoapOptList::new();
    optlist.add_uri_path_segments(&uri)?;

    let mut session = context.new_session(
        config.ip.parse().expect("Invalid IP"),
        uri,
        &config.user,
        &config.key,
        true,
    )?;

    session.request_status("65539")?;
    // session.update_light("65539", LightInfo::new().color_preset(LightColorPreset::Yellow))?;
    context.run(IO_FREQUENCY, Some(Box::new(handle_response)))?;
    
    // let pdu = CoapPduBuilder::new(CoapMethod::Put).with_optlist(&optlist);

    // const STEPS: usize = 2;
    // for i in 0..=STEPS {
    //     let hsv = Hsv::new(Deg(359.99 * (i as f32) / (STEPS as f32)), 1.0f32, 1.0f32);
    //     let light_control = LightInfo::new()
    //         .on(true)
    //         .brightness(254)
    //         .color_rgb(&Rgb::from_color(&hsv));

    //     let _token = session.send_pdu(pdu.with_payload(light_control))?;
    //     context.run(IO_FREQUENCY, Some(Box::new(handle_response)))?;
    //     sleep(Duration::from_secs(2));
    // }

    Ok(())
}

fn handle_response(_token: CoapToken, data: serde_json::Value) {
    dbg!(data);
}