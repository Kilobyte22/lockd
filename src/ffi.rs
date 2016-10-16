use std::ptr;
use std::ffi::CString;
use libc::{c_void, c_char, c_int};

#[link(name="X11")]
extern {
    fn XOpenDisplay(name: *const c_char) -> *mut c_void;
    fn XCloseDisplay(display: *mut c_void) -> c_int;
}

#[link(name="Xext")]
extern {
    fn DPMSGetTimeouts(display: *const c_void, standby: *mut u16, suspend: *mut u16, poweroff: *mut u16) -> c_int;
    fn DPMSSetTimeouts(display: *mut c_void, standby: u16, suspend: u16, poweroff: u16) -> u32;
}

pub struct Display {
    handle: *mut c_void
}

impl Display {

    pub fn new(name: Option<&str>) -> Option<Display> {
        let handle = match name {
            Some(name) => {
                let cname = CString::new(name).expect("Nul byte was encountered in display name");
                unsafe {
                    XOpenDisplay(cname.as_ptr())
                }
            },
            None => {
                unsafe {
                    XOpenDisplay(ptr::null())
                }
            }
        };
        if handle.is_null() {
            None
        } else {
            Some(Display {
                handle: handle
            })
        }
    }

    pub fn dpms_set_timeouts(&mut self, timeouts: (u16, u16, u16)) -> bool {
        unsafe {
            DPMSSetTimeouts(self.handle, timeouts.0, timeouts.1, timeouts.2) != 0
        }
    }

    pub fn dpms_get_timeouts(&self) -> Option<(u16, u16, u16)> {
        unsafe {
            let mut ret = (0, 0, 0);
            if DPMSGetTimeouts(self.handle, &mut ret.0, &mut ret.1, &mut ret.2) != 0 {
                Some(ret) 
            } else {
                None
            }
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            XCloseDisplay(self.handle);
        }
    }
}
