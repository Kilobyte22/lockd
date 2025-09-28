use std::io;

pub fn kill_process(pid: u32) -> io::Result<()> {
    unsafe {
        if libc::kill(pid as libc::pid_t, libc::SIGTERM) == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
