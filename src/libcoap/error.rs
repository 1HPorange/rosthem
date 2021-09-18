#[derive(Debug)]
pub enum CoapError {
    AlreadyInitialized,
    FailedToCreateContext,
    FailedToCreateSession,
    IdentityNotAscii,
    KeyNotAscii,
    InvalidUri,
    FailedToCreatePdu,
    FailedToSend,
    IoError,
    UriTooLong,
    SerializeError,
    AlreadyHasPayload,
}
