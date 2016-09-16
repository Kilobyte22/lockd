use std::process::{Command, Child};
use std::sync::mpsc::{Sender, Receiver};
use std::thread;

use msg::{LockMessage, CoreMessage};

pub fn actor_lockscreen(core: Sender<CoreMessage>, cmd: Receiver<LockMessage>) {
    let mut command = (format!("echo"), vec![format!("Error: did not provide correct lock command from core. This IS a bug.")]);
    let mut pid: Option<u32> = None;
    for message in cmd {
        match message {
            LockMessage::Lock => {
                let child = lock_command(&command);
                pid = Some(child.id());
                core.send(CoreMessage::Locked).unwrap();
                let core_clone = core.clone();
                thread::spawn(||{
                    actor_lock_instance(core_clone, child);
                });
            },
            LockMessage::Unlock => match pid {
                Some(pid) => {
                    unsafe { ::libc::kill(pid as i32, ::libc::SIGTERM) };
                },
                None => {}
            },
            LockMessage::SetLockscreen(cmd, params) => {
                command = (cmd, params);
            }
        }
    }
}

fn lock_command(command: &(String, Vec<String>)) -> Child {
    let mut c = Command::new(&command.0);
    //let c = command.1.iter().fold(c, |c, arg| c.arg(arg));
    // FIXME: Ugly workaround until i can get fold to behave
    for arg in &command.1 {
        c.arg(arg);
    }
    c.spawn().unwrap()
}
    
fn actor_lock_instance(core: Sender<CoreMessage>, mut child: Child) {
    child.wait().unwrap();
    core.send(CoreMessage::Unlocked).unwrap();
}
