use std::{convert::From, ffi::CString, mem, os::raw::c_char};
use bytes::Bytes;
use http::{Method, request::Builder};
use serde_json::{from_str as from_json, Value};
use wasi_experimental_http::request;

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn _start(ptr: *mut u8, size: usize) -> *mut c_char {
    let event_str;
    unsafe {
        let data = Vec::from_raw_parts(ptr, size, size);
        // read a Rust `String` from the byte array,
        event_str = String::from_utf8(data).unwrap();
    }
    let json = from_json::<Value>(event_str.as_str()).unwrap();
    println!("{:?}", json);
    let url = "https://postman-echo.com/post".to_string();
    let req = Builder::new()
        .method(Method::POST)
        .uri(&url)
        .header("Content-Type", "text/plain")
        .header("abc", "def");
    let b = Bytes::from("Testing");
    let req = req.body(Some(b)).unwrap();
    println!("{:?}", req);

    let mut res = request(req).expect("cannot make request");
    // println!("{:?}", res.into());
    let str = std::str::from_utf8(&res.body_read_all().unwrap()).unwrap().to_owned();
    println!("{:?}", str);
    println!("{:#?}", res.header_get("content-type".to_string()).unwrap());
    let status_code = res.status_code;
    println!("{:#?}", status_code);
    // let input_str = "whatever".to_string();
    let s = CString::new(str).unwrap();
    let ptr = s.as_ptr();
    println!("{:?}", ptr);
    // mem::forget(s);
    s.into_raw()
    // Tuple {
    //     ptr,
    //     size,
    // }
}

/// Allocate memory into the module's linear memory
/// and return the offset to the start of the block.
#[no_mangle]
pub fn alloc(size: usize) -> *mut u8 {
    // create a new mutable buffer with capacity `len`
    let mut buf = Vec::with_capacity(size);
    // take a mutable pointer to the buffer
    let ptr = buf.as_mut_ptr();
    // take ownership of the memory block and
    // ensure the its destructor is not
    // called when the object goes out of scope
    // at the end of the function
    mem::forget(buf);
    // return the pointer so the runtime
    // can write data at this offset
    ptr
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn dealloc(ptr: *mut c_char) {
    let _ = unsafe { CString::from_raw(ptr) };
}
