mod lambda {
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Error {
        ClientError,
        ServerError,
    }
    impl std::fmt::Debug for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::ClientError => f.debug_tuple("Error::ClientError").finish(),
                Error::ServerError => f.debug_tuple("Error::ServerError").finish(),
            }
        }
    }
    pub type Event = String;
    pub type Output = String;
    pub type Context = String;
    #[export_name = "handler"]
    unsafe extern "C" fn __wit_bindgen_handler(
        arg0: i32,
        arg1: i32,
        arg2: i32,
        arg3: i32,
        arg4: i32,
    ) -> i32 {
        let len0 = arg1 as usize;
        let result = <super::Lambda as Lambda>::handler(
            String::from_utf8(Vec::from_raw_parts(arg0 as *mut _, len0, len0)).unwrap(),
            match arg2 {
                0 => None,
                1 => Some({
                    let len1 = arg4 as usize;

                    String::from_utf8(Vec::from_raw_parts(arg3 as *mut _, len1, len1)).unwrap()
                }),
                _ => panic!("invalid enum discriminant"),
            },
        );
        let ptr2 = RET_AREA.0.as_mut_ptr() as i32;
        match result {
            Ok(e) => {
                *((ptr2 + 0) as *mut u8) = (0i32) as u8;
                let vec3 = (e.into_bytes()).into_boxed_slice();
                let ptr3 = vec3.as_ptr() as i32;
                let len3 = vec3.len() as i32;
                core::mem::forget(vec3);
                *((ptr2 + 8) as *mut i32) = len3;
                *((ptr2 + 4) as *mut i32) = ptr3;
            }
            Err(e) => {
                *((ptr2 + 0) as *mut u8) = (1i32) as u8;
                *((ptr2 + 4) as *mut u8) = (match e {
                    Error::ClientError => 0,
                    Error::ServerError => 1,
                }) as u8;
            }
        };
        ptr2
    }
    pub trait Lambda {
        fn handler(event: Event, context: Option<Context>) -> Result<Output, Error>;
    }

    #[repr(align(4))]
    struct RetArea([u8; 12]);
    static mut RET_AREA: RetArea = RetArea([0; 12]);
}
