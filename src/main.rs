use std::{net::Ipv4Addr, thread::sleep, time::Duration};

use prisma::{FromColor, Hsv, Rgb};
use rosthem::*;

struct Config {
    uri: String,
    ip: String,
    user: String,
    key: String,
}

fn main() -> Result<(), CoapError> {
    const IO_FREQUENCY: Option<Duration> = Some(Duration::from_millis(100));

    let config: Config =
        serde_json::from_str(std::fs::read("/config.json").expect("Missing config"))
            .expect("Invalid config");

    let coap = Coap::new(None)?;
    let context = coap.new_context()?;

    let uri = CoapUri::new(config.uri)?;
    let optlist = CoapOptList::new();
    optlist.add_uri_path_segments(&uri)?;

    let session = context.new_session(
        config.ip.parse().expect("Invalid IP"),
        &config.uri,
        &config.user,
        &config.key,
        true,
    )?;

    let pdu = CoapPduBuilder::new(&session, CoapMethod::Put).with_optlist(&optlist);

    const STEPS: usize = 8;
    for i in 0..=STEPS {
        let hsv = Hsv::new(Deg(359.99 * (i as f32) / (STEPS as f32)), 1.0f32, 1.0f32);
        let light_control = LightInfo::new()
            .on(true)
            .brightness(254)
            .color_rgb(&Rgb::from_color(&hsv));

        let pdu = pdu.build_with_payload(light_control)?;
        session.send_pdu(pdu)?;
        context.run(IO_FREQUENCY)?;
        sleep(Duration::from_secs(2));
    }

    Ok(())
}
