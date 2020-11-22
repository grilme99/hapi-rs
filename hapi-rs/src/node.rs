use super::errors::*;
use crate::auto::bindings as ffi;
pub use crate::auto::rusty::NodeType;
use crate::auto::rusty::{State, StatusType};
use crate::char_ptr;
use crate::session::Session;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::null;
use std::sync::Arc;
use std::task::{Context, Poll};

#[cfg(feature = "async")]
mod async_ {
    use super::*;
    pub struct CookFuture {
        node_id: i32,
        session: Session,
    }

    impl CookFuture {
        pub fn new(node_id: i32, session: Session) -> CookFuture {
            unsafe {
                let r = ffi::HAPI_CookNode(session.ptr(), node_id, null());
                eprintln!("Starting async cooking...");
                assert!(matches!(r, ffi::HAPI_Result::HAPI_RESULT_SUCCESS));
            }
            CookFuture { node_id, session }
        }

        // pub fn complete(&self) -> std::result::Result<(), ()> {
        //     eprintln!("Starting cooking");
        //     loop {
        //         match self.state() {
        //             State::StateReady => break Ok(()),
        //             State::StateCooking | State::StartingCook => {
        //             }
        //             State::CookErrors => break Err(()),
        //             _s => {}
        //         }
        //     }
        // }
    }

    impl std::future::Future for CookFuture {
        type Output = Result<State>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.session.get_status(StatusType::CookState) {
                Err(e) => panic!("Temporary"),
                Ok(s) => match s {
                    State::StateReady => Poll::Ready(Ok(State::StateReady)),
                    State::StateCooking | State::StartingCook => {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    State::CookErrors => {
                        Poll::Ready(Err(HapiError::new(Kind::CookError, None, None)))
                    }
                    _s => Poll::Ready(Err(HapiError::new(Kind::CookError, None, None))),
                },
            }
        }
    }
}
#[derive(Debug)]
#[non_exhaustive]
pub enum HoudiniNode {
    SopNode(SopNode),
    ObjNode(ObjNode),
}

impl HoudiniNode {
    pub fn delete(self) -> Result<()> {
        use HoudiniNode::*;
        let (id, session) = match &self {
            SopNode(n) => (n.id, n.session.ptr()),
            ObjNode(n) => (n.id, n.session.ptr()),
        };
        unsafe {
            let mut info = MaybeUninit::uninit();
            ffi::HAPI_GetNodeInfo(session, id, info.as_mut_ptr()).result(session, None)?;
            let info = info.assume_init();
            // if info.createdPostAssetLoad != 0 {
            //     unimplemented!()
            // }
            ffi::HAPI_DeleteNode(session, id).result(session, None)
        }
    }

    #[inline]
    fn strip(&self) -> (ffi::HAPI_NodeId, &Session) {
        match &self {
            HoudiniNode::SopNode(n) => (n.id, &n.session),
            HoudiniNode::ObjNode(n) => (n.id, &n.session),
        }
    }

    #[cfg(feature = "async")]
    pub fn cook(&self) -> async_::CookFuture {
        let (id, session) = self.strip();
        debug_assert!(session.unsync, "Session is sync!");
        async_::CookFuture::new(id, session.clone())
    }

    pub fn cook_blocking(&self) -> Result<()> {
        let (id, session) = self.strip();
        debug_assert!(!session.unsync, "Session is async!");
        unsafe { ffi::HAPI_CookNode(session.ptr(), id, null()).result(session.ptr(), None) }
    }

    pub fn create(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: Session,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.strip().0);
        let mut label_ptr: *const std::os::raw::c_char = null();
        unsafe {
            let mut tmp;
            if let Some(lb) = label {
                tmp = CString::from_vec_unchecked(lb.into());
                label_ptr = tmp.as_ptr();
            }
            let name = CString::from_vec_unchecked(name.into());
            ffi::HAPI_CreateNode(
                session.ptr(),
                parent,
                name.as_ptr(),
                label_ptr,
                cook as i8,
                id.as_mut_ptr(),
            )
            .result(session.ptr(), None)?;
            Ok(HoudiniNode::ObjNode(ObjNode {
                id: id.assume_init(),
                session,
            }))
        }
    }
}

#[derive(Debug)]
pub struct SopNode {
    id: ffi::HAPI_NodeId,
    session: Session,
}
#[derive(Debug)]
pub struct ObjNode {
    id: ffi::HAPI_NodeId,
    session: Session,
}

impl SopNode {
}

impl ObjNode {
}
