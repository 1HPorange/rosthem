pub mod error;
mod ffi;
pub mod light;

use self::error::CoapError;
use ffi::*;
use serde::Serialize;
use std::{
    net::Ipv4Addr,
    pin::Pin,
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static COAP_INITIALIZED: AtomicBool = AtomicBool::new(false);

// TODO: Look at the return values of every native function, split errors better

#[repr(i32)]
#[derive(Copy, Clone)]
pub enum CoapLogLevel {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

pub struct Coap {
    // Prevent this thing from doing any multithreading magic because I don't know if libcoap likes that
    _private: *const u8,
}

impl Drop for Coap {
    fn drop(&mut self) {
        unsafe {
            coap_cleanup();
            COAP_INITIALIZED.store(false, Ordering::SeqCst);
        }
    }
}

impl Coap {
    pub fn new(log_level: Option<CoapLogLevel>) -> Result<Coap, CoapError> {
        if let Ok(false) =
            COAP_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            unsafe {
                coap_startup();

                let log_level = log_level.unwrap_or(CoapLogLevel::Emergency);
                coap_dtls_set_log_level(log_level as i32);
                coap_set_log_level(log_level as u32);
            }
            Ok(Coap {
                _private: ptr::null(),
            })
        } else {
            Err(CoapError::AlreadyInitialized)
        }
    }

    pub fn new_context<'a>(&'a self) -> Result<CoapContext<'a>, CoapError> {
        unsafe {
            let ctx = coap_new_context(ptr::null());
            // coap_context_set_keepalive(ctx, ping_seconds);
            // coap_context_set_block_mode(ctx, block_mode);
            if let Some(ctx) = NonNull::new(ctx) {
                coap_context_set_block_mode(
                    ctx.as_ptr(),
                    COAP_BLOCK_USE_LIBCOAP as u8 | COAP_BLOCK_SINGLE_BODY as u8,
                );
                coap_register_option(ctx.as_ptr(), COAP_OPTION_BLOCK2 as u16);
                coap_register_response_handler(ctx.as_ptr(), Some(handle_response));
                coap_register_event_handler(ctx.as_ptr(), Some(handle_event));
                coap_register_nack_handler(ctx.as_ptr(), Some(handle_nack));
                Ok(CoapContext {
                    inner: ctx,
                    _coap: self,
                })
            } else {
                Err(CoapError::FailedToCreateContext)
            }
        }
    }
}

pub struct CoapContext<'a> {
    inner: NonNull<coap_context_t>,
    _coap: &'a Coap,
}

impl Drop for CoapContext<'_> {
    fn drop(&mut self) {
        unsafe {
            coap_free_context(self.inner.as_ptr());
        }
    }
}

impl CoapContext<'_> {
    // TODO: Parse ip from uri
    pub fn new_session(
        &self,
        ip: Ipv4Addr,
        uri: CoapUri,
        identity: &str,
        key: &str,
        warmup: bool,
    ) -> Result<CoapSession, CoapError> {
        unsafe {
            let server = CoapAddress::new(ip);
            let mut dtls_psk = CoapDtlsPsk::new(uri, identity, key)?;
            let session = coap_new_client_session_psk2(
                self.inner.as_ptr(),
                ptr::null(),
                &server.native,
                coap_proto_t_COAP_PROTO_DTLS,
                &mut Pin::get_unchecked_mut(dtls_psk.as_mut()).native,
            );

            let coap_session = NonNull::new(session)
                .ok_or(CoapError::FailedToCreateSession)
                .map(|inner| CoapSession {
                    inner,
                    _context: self,
                })?;

            if warmup {
                self.run(Some(Duration::from_millis(1500)))?; // TODO: Is this number sensible?
            }

            Ok(coap_session)
        }
    }

    pub fn run(&self, timeout_ms: Option<Duration>) -> Result<(), CoapError> {
        let ffi_timeout_ms = timeout_ms.map(|d| d.as_millis() as u32).unwrap_or(0); // The special value of 0 means block until IO is available (so potentially forever)
        loop {
            let result = unsafe { coap_io_process(self.inner.as_ptr(), ffi_timeout_ms) };

            if result == -1 {
                return Err(CoapError::IoError);
            } else if timeout_ms
                .map(|timeout_ms| (result - timeout_ms.as_millis() as i32).abs() < 5)
                .unwrap_or(false)
            {
                return Ok(());
            }
        }
    }
}

pub struct CoapSession<'a> {
    inner: NonNull<coap_session_t>,
    _context: &'a CoapContext<'a>,
}

impl Drop for CoapSession<'_> {
    fn drop(&mut self) {
        unsafe { coap_session_release(self.inner.as_ptr()) }
    }
}

impl CoapSession<'_> {
    pub fn send_pdu(&self, pdu: CoapPdu) -> Result<(), CoapError> {
        unsafe {
            coap_send(self.inner.as_ptr(), pdu.inner.as_ptr());
            Ok(())
        }
    }
}

struct CoapDtlsPsk {
    uri: CoapUri,
    native: coap_dtls_cpsk_t,
}

impl CoapDtlsPsk {
    fn new(uri: CoapUri, identity: &str, key: &str) -> Result<Pin<Box<CoapDtlsPsk>>, CoapError> {
        if !identity.is_ascii() {
            return Err(CoapError::IdentityNotAscii);
        }

        if !key.is_ascii() {
            return Err(CoapError::KeyNotAscii);
        }

        unsafe {
            let mut dtls_psk: coap_dtls_cpsk_t = std::mem::zeroed();

            dtls_psk.version = COAP_DTLS_CPSK_SETUP_VERSION as u8;

            dtls_psk.validate_ih_call_back = None;

            dtls_psk.psk_info.identity.s = identity.as_ptr();
            dtls_psk.psk_info.identity.length = identity.len() as u32;
            dtls_psk.psk_info.key.s = key.as_ptr();
            dtls_psk.psk_info.key.length = key.len() as u32;

            let mut psk = Pin::new(Box::new(CoapDtlsPsk {
                uri,
                native: dtls_psk,
            }));
            psk.native.ih_call_back_arg = &mut psk.native.psk_info as *mut _ as *mut _;
            psk.native.client_sni = psk.uri.native.host.s as *mut _;

            return Ok(psk);
        }
    }
}

#[derive(Debug)]
pub struct CoapUri {
    uri: String,
    native: coap_uri_t,
}

impl Clone for CoapUri {
    fn clone(&self) -> Self {
        unsafe {
            let mut uri = CoapUri {
                uri: self.uri.clone(),
                native: std::mem::zeroed(),
            };
            coap_split_uri(uri.uri.as_ptr(), uri.uri.len() as u32, &mut uri.native);
            uri
        }
    }
}

impl CoapUri {
    pub fn new(uri: String) -> Result<CoapUri, CoapError> {
        unsafe {
            let mut uri = CoapUri {
                uri,
                native: std::mem::zeroed(),
            };

            if coap_split_uri(uri.uri.as_ptr(), uri.uri.len() as u32, &mut uri.native) == 0 {
                Ok(uri)
            } else {
                Err(CoapError::InvalidUri)
            }
        }
    }
}

pub struct CoapAddress {
    native: coap_address_t,
}

impl CoapAddress {
    // TODO: Support IPv6
    pub fn new(ip: Ipv4Addr) -> CoapAddress {
        let b = ip.octets();

        CoapAddress {
            native: unsafe {
                std::mem::transmute([
                    //                                   PORT HL ---  IP -------------------
                    0x10, 0x00, 0x00, 0x00, 0x02, 0x00, 0x16, 0x34, b[0], b[1], b[2], b[3], 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x00, 0x00, 0x00, 0xE0, 0x00,
                    0x00, 0x00, 0x06, 0x00, 0x00, 0x00u8,
                ])
            },
        }
    }
}

pub struct CoapOptList {
    inner: *mut coap_optlist_t,
}

impl Drop for CoapOptList {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                coap_delete_optlist(self.inner);
            }
        }
    }
}

impl CoapOptList {
    pub(crate) fn new() -> Self {
        Self {
            inner: ptr::null_mut(),
        }
    }

    pub(crate) fn add_uri_path_segments(&self, uri: &CoapUri) -> Result<(), CoapError> {
        unsafe {
            if uri.native.path.length > 0 {
                let mut uri_path_buf_len = [0u8; 256]; // PERF: Uninitialized

                if uri.native.path.length > uri_path_buf_len.len() as u32 {
                    return Err(CoapError::UriTooLong);
                }

                let mut _used_buf_len = uri_path_buf_len.len() as u32;
                let path_segment_count = coap_split_path(
                    uri.native.path.s,
                    uri.native.path.length,
                    uri_path_buf_len.as_mut_ptr(),
                    &mut _used_buf_len,
                );

                let mut buf_write_offset = 0usize;
                let mut writable_buf = &mut uri_path_buf_len[..];
                for _ in 0..path_segment_count {
                    // println!("Path segment: {:?}", std::str::from_utf8_unchecked(std::slice::from_raw_parts(coap_opt_value(writable_buf.as_mut_ptr()), coap_opt_length(writable_buf.as_mut_ptr()) as usize)));

                    coap_insert_optlist(
                        &self.inner as *const _ as *mut _,
                        coap_new_optlist(
                            COAP_OPTION_URI_PATH as u16,
                            coap_opt_length(writable_buf.as_mut_ptr()),
                            coap_opt_value(writable_buf.as_mut_ptr()),
                        ),
                    );

                    buf_write_offset += coap_opt_size(writable_buf.as_mut_ptr()) as usize;
                    writable_buf = &mut uri_path_buf_len[buf_write_offset..];
                }
            }

            Ok(())
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CoapMethod {
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4,
    Fetch = 5,
    Patch = 6,
    Ipatch = 7,
}

pub struct CoapPdu<'a> {
    inner: NonNull<coap_pdu_t>,
    session: &'a CoapSession<'a>,
    has_payload: bool,
}

// TODO: Investigate double free, potentially because of the optlist drop? or payload?
// impl Drop for CoapPdu<'_> {
//     fn drop(&mut self) {
//         unsafe {
//             coap_delete_pdu(self.inner.as_ptr());
//         }
//     }
// }

impl<'a> CoapPdu<'a> {
    fn new(session: &'a CoapSession<'a>, method: CoapMethod) -> Result<CoapPdu<'a>, CoapError> {
        unsafe {
            NonNull::new(coap_new_pdu(
                coap_pdu_type_t_COAP_MESSAGE_CON,
                method as u32,
                session.inner.as_ptr(),
            ))
            .map(|inner| CoapPdu {
                inner,
                session,
                has_payload: false,
            })
            .ok_or(CoapError::FailedToCreatePdu)
        }
    }

    fn add_payload<P: Serialize>(&mut self, payload: P) -> Result<(), CoapError> {
        if self.has_payload {
            return Err(CoapError::AlreadyHasPayload);
        } else {
            self.has_payload = true;
        }

        unsafe {
            let payload = serde_json::to_string(&payload)
                .map(|json| Box::new(json.into_bytes()))
                .map_err(|_| CoapError::SerializeError)?;

            let payload_ptr = payload.as_ptr();
            let payload_len = payload.len();
            let payload_vec = Box::into_raw(payload);
            coap_add_data_large_request(
                self.session.inner.as_ptr(),
                self.inner.as_ptr(),
                payload_len as u32,
                payload_ptr,
                Some(drop_boxed_slice),
                payload_vec as _,
            );

            Ok(())
        }
    }

    fn add_optlist(&self, optlist: &CoapOptList) -> Result<(), CoapError> {
        unsafe {
            coap_add_optlist_pdu(self.inner.as_ptr(), &optlist.inner as *const _ as *mut _); // TODO: Failure case
            Ok(())
        }
    }
}

#[derive(Copy, Clone)]
pub struct CoapPduBuilder<'a> {
    session: &'a CoapSession<'a>,
    method: CoapMethod,
    optlist: Option<&'a CoapOptList>,
}

impl<'a> CoapPduBuilder<'a> {
    pub fn new(session: &'a CoapSession, method: CoapMethod) -> CoapPduBuilder<'a> {
        CoapPduBuilder {
            session,
            method,
            optlist: None,
        }
    }

    pub fn with_optlist(mut self, optlist: &'a CoapOptList) -> CoapPduBuilder<'a> {
        self.optlist = Some(optlist);
        self
    }

    pub fn build(self) -> Result<CoapPdu<'a>, CoapError> {
        let pdu = CoapPdu::new(self.session, self.method)?;
        if let Some(optlist) = self.optlist {
            pdu.add_optlist(optlist)?;
        }
        Ok(pdu)
    }

    pub fn build_with_payload<P: Serialize>(self, payload: P) -> Result<CoapPdu<'a>, CoapError> {
        let mut pdu = self.build()?;
        pdu.add_payload(payload)?;
        Ok(pdu)
    }
}

unsafe extern "C" fn handle_response(
    _session: *mut coap_session_t,
    _sent: *const coap_pdu_t,
    received: *const coap_pdu_t,
    _mid: coap_mid_t,
) -> coap_response_t {
    let _rcv_code = coap_pdu_get_code(received);
    let _rcv_type = coap_pdu_get_type(received);

    // println!("A got sometin");
    // coap_show_pdu(coap_log_t_LOG_DEBUG, received);
    return coap_response_t_COAP_RESPONSE_OK;

    // coap_log(LOG_DEBUG, "** process incoming %d.%02d response:\n",
    //          COAP_RESPONSE_CLASS(rcv_code), rcv_code & 0x1F);
    // if (coap_get_log_level() < LOG_DEBUG)
    //   coap_show_pdu(LOG_INFO, received);

    // /* check if this is a response to our original request */
    // if (!check_token(received)) {
    //   /* drop if this was just some message, or send RST in case of notification */
    //   if (!sent && (rcv_type == COAP_MESSAGE_CON ||
    //                 rcv_type == COAP_MESSAGE_NON)) {
    //     /* Cause a CoAP RST to be sent */
    //     return COAP_RESPONSE_FAIL;
    //   }
    //   return COAP_RESPONSE_OK;
    // }

    // if (rcv_type == COAP_MESSAGE_RST) {
    //   coap_log(LOG_INFO, "got RST\n");
    //   return COAP_RESPONSE_OK;
    // }

    // /* output the received data, if any */
    // if (COAP_RESPONSE_CLASS(rcv_code) == 2) {

    //   /* set obs timer if we have successfully subscribed a resource */
    //   if (doing_observe && !obs_started &&
    //       coap_check_option(received, COAP_OPTION_OBSERVE, &opt_iter)) {
    //     coap_log(LOG_DEBUG,
    //              "observation relationship established, set timeout to %d\n",
    //              obs_seconds);
    //     obs_started = 1;
    //     obs_ms = obs_seconds * 1000;
    //     obs_ms_reset = 1;
    //   }

    //   if (coap_get_data_large(received, &len, &databuf, &offset, &total)) {
    //     append_to_output(databuf, len);
    //     if ((len + offset == total) && add_nl)
    //       append_to_output((const uint8_t*)"\n", 1);
    //   }

    //   /* Check if Block2 option is set */
    //   block_opt = coap_check_option(received, COAP_OPTION_BLOCK2, &opt_iter);
    //   if (!single_block_requested && block_opt) { /* handle Block2 */

    //     /* TODO: check if we are looking at the correct block number */
    //     if (coap_opt_block_num(block_opt) == 0) {
    //       /* See if observe is set in first response */
    //       ready = doing_observe ? coap_check_option(received,
    //                                 COAP_OPTION_OBSERVE, &opt_iter) == NULL : 1;
    //     }
    //     if(COAP_OPT_BLOCK_MORE(block_opt)) {
    //       wait_ms = wait_seconds * 1000;
    //       wait_ms_reset = 1;
    //       doing_getting_block = 1;
    //     }
    //     else {
    //       doing_getting_block = 0;
    //     }
    //     return COAP_RESPONSE_OK;
    //   }
    // } else {      /* no 2.05 */
    //   /* check if an error was signaled and output payload if so */
    //   if (COAP_RESPONSE_CLASS(rcv_code) >= 4) {
    //     fprintf(stderr, "%d.%02d", COAP_RESPONSE_CLASS(rcv_code),
    //             rcv_code & 0x1F);
    //     if (coap_get_data_large(received, &len, &databuf, &offset, &total)) {
    //       fprintf(stderr, " ");
    //       while(len--) {
    //         fprintf(stderr, "%c", isprint(*databuf) ? *databuf : '.');
    //         databuf++;
    //       }
    //     }
    //     fprintf(stderr, "\n");
    //   }

    // }

    // /* our job is done, we can exit at any time */
    // ready = doing_observe ? coap_check_option(received,
    //                                 COAP_OPTION_OBSERVE, &opt_iter) == NULL : 1;
    // return COAP_RESPONSE_OK;
}

unsafe extern "C" fn handle_event(
    _session: *mut coap_session_t,
    _event: coap_event_t,
) -> ::std::os::raw::c_int {
    // match event {
    //     coap_event_t_COAP_EVENT_SESSION_CLOSED => {
    //         println!("coap_event_t_COAP_EVENT_SESSION_CLOSED")
    //     } // TODO: quit
    //     coap_event_t_COAP_EVENT_DTLS_CLOSED => println!("coap_event_t_COAP_EVENT_DTLS_CLOSED"),
    //     coap_event_t_COAP_EVENT_TCP_CLOSED => println!("coap_event_t_COAP_EVENT_TCP_CLOSED"),
    //     coap_event_t_COAP_EVENT_DTLS_CONNECTED => {
    //         println!("coap_event_t_COAP_EVENT_DTLS_CONNECTED")
    //     }
    //     coap_event_t_COAP_EVENT_DTLS_RENEGOTIATE => {
    //         println!("coap_event_t_COAP_EVENT_DTLS_RENEGOTIATE")
    //     }
    //     coap_event_t_COAP_EVENT_DTLS_ERROR => println!("coap_event_t_COAP_EVENT_DTLS_ERROR"),
    //     coap_event_t_COAP_EVENT_TCP_CONNECTED => println!("coap_event_t_COAP_EVENT_TCP_CONNECTED"),
    //     coap_event_t_COAP_EVENT_TCP_FAILED => println!("coap_event_t_COAP_EVENT_TCP_FAILED"),
    //     coap_event_t_COAP_EVENT_SESSION_CONNECTED => {
    //         println!("coap_event_t_COAP_EVENT_SESSION_CONNECTED")
    //     }
    //     coap_event_t_COAP_EVENT_SESSION_FAILED => {
    //         println!("coap_event_t_COAP_EVENT_SESSION_FAILED")
    //     }
    //     coap_event_t_COAP_EVENT_PARTIAL_BLOCK => println!("coap_event_t_COAP_EVENT_PARTIAL_BLOCK"),
    //     _ => {}
    // }

    return 0;
}

unsafe extern "C" fn handle_nack(
    _session: *mut coap_session_t,
    _sent: *const coap_pdu_t,
    _reason: coap_nack_reason_t,
    _mid: coap_mid_t,
) {
    // match reason {
    //     coap_nack_reason_t_COAP_NACK_TLS_FAILED => {
    //         println!("coap_nack_reason_t_COAP_NACK_TLS_FAILED")
    //     } // TODO: quit
    //     coap_nack_reason_t_COAP_NACK_TOO_MANY_RETRIES => {
    //         println!("coap_nack_reason_t_COAP_NACK_TOO_MANY_RETRIES")
    //     }
    //     coap_nack_reason_t_COAP_NACK_NOT_DELIVERABLE => {
    //         println!("coap_nack_reason_t_COAP_NACK_NOT_DELIVERABLE")
    //     }
    //     coap_nack_reason_t_COAP_NACK_RST => println!("coap_nack_reason_t_COAP_NACK_RST"),
    //     coap_nack_reason_t_COAP_NACK_ICMP_ISSUE => {
    //         println!("coap_nack_reason_t_COAP_NACK_ICMP_ISSUE")
    //     }
    //     _ => {}
    // }
}

unsafe extern "C" fn drop_boxed_slice(
    _session: *mut coap_session_t,
    app_ptr: *mut ::std::os::raw::c_void,
) {
    drop(Box::<Vec<u8>>::from_raw(app_ptr as _));
}
