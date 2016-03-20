use std::process::{Command, Child};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;

use msg::{LockMessage, InhibitMessage, CoreMessage};

pub fn actor_lockscreen(core: Sender<CoreMessage>, cmd: Receiver<LockMessage>) {
  let mut pid: Option<u32> = None;
  for message in cmd {
    match message {
      LockMessage::Lock => {
        let child = lock_command();
        pid = Some(child.id());
        core.send(CoreMessage::Locked);
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
      }
    }
  }
}

fn lock_command() -> Child {
  let mut c = Command::new("i3lock");
  c.arg("-c").arg("000000").arg("--nofork");
  c.spawn().unwrap()
}
  
fn actor_lock_instance(core: Sender<CoreMessage>, mut child: Child) {
  child.wait();
  core.send(CoreMessage::Unlocked);
}
