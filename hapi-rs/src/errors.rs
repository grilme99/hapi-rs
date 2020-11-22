use crate::{
    auto::bindings as ffi,
    check_session
};
use std::cell::Cell;

// TODO: Rethink the design. Passing raw pointer to session may be not a good idea
pub type Result<T> = std::result::Result<T, HapiError>;

#[derive(Debug)]
pub struct HapiError {
    pub kind: Kind,
    pub(crate) session: Option<Cell<*const ffi::HAPI_Session>>,
    pub message: Option<&'static str>,
}

#[derive(Debug)]
pub enum Kind {
    Hapi(ffi::HAPI_Result),
    CookError,
    NullByte,
}

impl Kind {
    fn description(&self) -> &str {
        use ffi::HAPI_Result::*;

        match self {
            Kind::Hapi(HAPI_RESULT_SUCCESS) => "SUCCESS",
            Kind::Hapi(HAPI_RESULT_FAILURE) => "FAILURE",
            Kind::Hapi(HAPI_RESULT_ALREADY_INITIALIZED) => "ALREADY_INITIALIZED",
            Kind::Hapi(HAPI_RESULT_NOT_INITIALIZED) => "NOT_INITIALIZED",
            Kind::Hapi(HAPI_RESULT_CANT_LOADFILE) => "CANT_LOADFILE",
            Kind::Hapi(HAPI_RESULT_PARM_SET_FAILED) => "PARM_SET_FAILED",
            Kind::Hapi(HAPI_RESULT_INVALID_ARGUMENT) => "PARM_SET_FAILED",
            Kind::Hapi(HAPI_RESULT_CANT_LOAD_GEO) => "CANT_LOAD_GEO",
            Kind::Hapi(HAPI_RESULT_CANT_GENERATE_PRESET) => "CANT_GENERATE_PRESET",
            Kind::Hapi(HAPI_RESULT_CANT_LOAD_PRESET) => "CANT_LOAD_PRESET",
            Kind::Hapi(HAPI_RESULT_ASSET_DEF_ALREADY_LOADED) => "ASSET_DEF_ALREADY_LOADED",
            Kind::Hapi(HAPI_RESULT_NO_LICENSE_FOUND) => "NO_LICENSE_FOUND",
            Kind::Hapi(HAPI_RESULT_DISALLOWED_NC_LICENSE_FOUND) => "DISALLOWED_NC_LICENSE_FOUND",
            Kind::Hapi(HAPI_RESULT_DISALLOWED_NC_ASSET_WITH_C_LICENSE) => {
                "DISALLOWED_NC_ASSET_WITH_C_LICENSE"
            }
            Kind::Hapi(HAPI_RESULT_DISALLOWED_NC_ASSET_WITH_LC_LICENSE) => {
                "DISALLOWED_NC_ASSET_WITH_LC_LICENSE"
            }
            Kind::Hapi(HAPI_RESULT_DISALLOWED_LC_ASSET_WITH_C_LICENSE) => {
                "DISALLOWED_LC_ASSET_WITH_C_LICENSE"
            }
            Kind::Hapi(HAPI_RESULT_DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN) => {
                "DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN"
            }
            Kind::Hapi(HAPI_RESULT_ASSET_INVALID) => "ASSET_INVALID",
            Kind::Hapi(HAPI_RESULT_NODE_INVALID) => "NODE_INVALID",
            Kind::Hapi(HAPI_RESULT_USER_INTERRUPTED) => "USER_INTERRUPTED",
            Kind::Hapi(HAPI_RESULT_INVALID_SESSION) => "INVALID_SESSION",
            Kind::NullByte => "String contains null byte!",
            Kind::CookError => "Cooking error",
            Kind::Hapi(_) => unreachable!(),
        }
    }
}

impl HapiError {
    pub fn new(
        kind: Kind,
        session: Option<*const ffi::HAPI_Session>,
        message: Option<&'static str>,
    ) -> HapiError {
        HapiError {
            kind,
            session: session.map(|s| Cell::new(s)),
            message,
        }
    }
}

impl std::fmt::Display for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        dbg!(self);
        match self.kind {
            Kind::Hapi(_) => {
                if let Some(session) = &self.session {
                    let session = session.get();
                    check_session!(session);
                    let mut tmp = None;
                    let last_error = if !session.is_null() {
                        tmp = Some(
                            get_last_error(session).expect("Could not retrieve last error"),
                        );
                        Some(tmp.as_ref().unwrap().as_str())
                    } else {
                        None
                    };
                    write!(
                        f,
                        "{}: {}",
                        self.kind.description(),
                        last_error.or(self.message).unwrap_or("Zig")
                    )
                } else {
                    write!(f, "{}", self.kind.description())
                }
            }
            _ => unreachable!(),
        }
    }
}

// TODO cooking errors
pub fn get_last_error(session: *const ffi::HAPI_Session) -> Result<String> {
    use ffi::HAPI_StatusType::HAPI_STATUS_CALL_RESULT;
    use ffi::HAPI_StatusVerbosity::HAPI_STATUSVERBOSITY_0;
    unsafe {
        let mut length = std::mem::MaybeUninit::uninit();
        let res = ffi::HAPI_GetStatusStringBufLength(
            session,
            HAPI_STATUS_CALL_RESULT,
            HAPI_STATUSVERBOSITY_0,
            length.as_mut_ptr(),
        )
        .result(session, Some("GetStatusStringBufLength failed"))?;
        let length = length.assume_init();
        let mut buf = vec![0u8; length as usize];
        ffi::HAPI_GetStatusString(
            session,
            HAPI_STATUS_CALL_RESULT,
            // SAFETY: casting to u8 to i8 (char)?
            buf.as_mut_ptr() as *mut i8,
            length,
        )
        .result(session, Some("GetStatusString failed"))?;
        buf.truncate(length as usize);
        Ok(String::from_utf8_unchecked(buf))
    }
}

impl From<std::ffi::NulError> for HapiError {
    fn from(_: std::ffi::NulError) -> Self {
        HapiError::new(Kind::NullByte, None, None)
    }
}

#[macro_export]
macro_rules! hapi_ok {
    ($hapi_result:expr, $session:expr, $message:expr) => {
        match $hapi_result {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(()),
            e => Err(HapiError::new(Kind::Hapi(e), Some($session), $message)),
        }
    };
}

#[macro_export]
macro_rules! hapi_err {
    ($hapi_result:expr, $session:expr, $message:expr) => {
        Err(HapiError::new(
            Kind::Hapi($hapi_result),
            Some($session),
            $expr,
        ))
    };

    ($hapi_result:expr) => {
        Err(HapiError::new(Kind::Hapi($hapi_result), None, None))
    };
}

impl std::error::Error for HapiError {}

impl ffi::HAPI_Result {
    pub(crate) fn result(
        &self,
        session: *const ffi::HAPI_Session,
        message: Option<&'static str>,
    ) -> Result<()> {
        hapi_ok!(*self, session, message)
    }
}
