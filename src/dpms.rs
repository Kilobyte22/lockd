use ffi;
use msg;
use std::sync::mpsc::{Sender, Receiver};

pub fn actor_dpms(core: Sender<msg::CoreMessage>, inbox: Receiver<msg::DPMSMessage>) {
    if let Some(mut display) = ffi::Display::new(None) {
        for msg in inbox {
            match msg {
                msg::DPMSMessage::SetValues((standby, suspend, poweroff)) => {
                    println!("Setting DPMS to {:?}", (standby, suspend, poweroff));
                    if !display.dpms_set_timeouts((standby, suspend, poweroff)) {
                        println!("Warning: Failed to set DPMS parameters");
                    }
                },
                msg::DPMSMessage::GetValues(reply) => {
                    reply.send(display.dpms_get_timeouts().expect("Internal xlib error while querying dpms values"));
                }
            }
        }
    }
}
