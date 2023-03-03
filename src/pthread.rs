use libc::pthread_t;
use std::os::raw::{c_char, c_int};
use std::ffi::CStr;
// uclibc doesn't have this function
#[no_mangle]
pub extern "C" fn pthread_setname_np(_thread: pthread_t, name: *const c_char) -> c_int {
    let name = unsafe { CStr::from_ptr(name) };
    eprintln!("TODO: implement pthread_setname_np. name: '{}'", name.to_string_lossy());
    0
}
