use crate::{CoapError, CoapOptList, CoapPduBuilder, CoapSession, LightInfo};

const IKEA_GATEWAY_PATH_SEGMENT: &'static str = "15001";

pub trait CoapSessionExt {
    fn get_status(&self, id: &'static str) -> Result<serde_json::Value, CoapError>;
}

impl CoapSessionExt for CoapSession {
    fn get_status(&self, id: &'static str) -> Result<serde_json::Value, CoapError> {
        let optlist = CoapOptList::new();
        optlist.add_path_segment(IKEA_GATEWAY_PATH_SEGMENT)?;
        optlist.add_path_segment(id)?;

        let pdu = CoapPduBuilder::new(&session, CoapMethod::Get)
            .with_optlist(&optlist)
            .build()?;

        self.send_pdu(&pdu)?;

        Ok(todo!())
    }

    fn update_light(&self, id: &'static str, command: LightInfo) -> Result<(), CoapError> {
        let optlist = CoapOptList::new();
        optlist.add_path_segment(IKEA_GATEWAY_PATH_SEGMENT)?;
        optlist.add_path_segment(id)?;

        let pdu = CoapPduBuilder::new(&session, CoapMethod::Put)
            .with_optlist(&optlist)
            .build_with_payload(command)?;

        self.send_pdu(&pdu)?;

        Ok(())
    }
}
