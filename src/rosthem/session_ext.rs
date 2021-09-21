use crate::{CoapError, CoapOptList, CoapPduBuilder, CoapSession, LightInfo, CoapMethod};

const IKEA_GATEWAY_PATH_SEGMENT: &'static str = "15001";

pub trait CoapSessionExt {
    fn request_status(&mut self, id: &'static str) -> Result<(), CoapError>;
    fn update_light(&mut self, id: &'static str, command: LightInfo) -> Result<(), CoapError>;
}

impl CoapSessionExt for CoapSession {
    fn request_status(&mut self, id: &'static str) -> Result<(), CoapError> {
        let optlist = CoapOptList::new();
        optlist.add_path_segment(IKEA_GATEWAY_PATH_SEGMENT)?;
        optlist.add_path_segment(id)?;

        let pdu = CoapPduBuilder::new(CoapMethod::Get)
            .with_optlist(&optlist);

        self.send_pdu(pdu)?;

        Ok(())
    }

    fn update_light(&mut self, id: &'static str, command: LightInfo) -> Result<(), CoapError> {
        let optlist = CoapOptList::new();
        optlist.add_path_segment(IKEA_GATEWAY_PATH_SEGMENT)?;
        optlist.add_path_segment(id)?;

        let pdu = CoapPduBuilder::new(CoapMethod::Put)
            .with_optlist(&optlist)
            .with_payload(command);

        self.send_pdu(pdu)?;

        Ok(())
    }
}
