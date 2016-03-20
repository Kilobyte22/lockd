extern crate toml;
extern crate libc;
extern crate dbus;

use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;

//mod config;
mod msg;
mod lockscreen;
mod inhibit;
mod react;
mod api;

use msg::{LockMessage, InhibitMessage, CoreMessage, CoreFlag};

struct ActorMainHandles {
  lockscreen: Sender<LockMessage>,
  inhibitors: Sender<InhibitMessage>,
  inbox: Receiver<CoreMessage>
}

struct State {
  locked: bool,
  inhibit_lid: bool,
  locking: bool,
  should_exit: bool
}

fn main() {
  let (core_send, core_recv) = mpsc::channel::<CoreMessage>();
  let (inh_send, inh_recv) = mpsc::channel::<InhibitMessage>();
  let (lock_send, lock_recv) = mpsc::channel::<LockMessage>();

  let core = core_send.clone();
  thread::spawn(||{
    lockscreen::actor_lockscreen(core, lock_recv);
  });
  let core = core_send.clone();
  thread::spawn(||{
    inhibit::actor_inhibit(core, inh_recv);
  });
  let core = core_send.clone();
  thread::spawn(||{
    react::actor_react(core);
  });
  let core = core_send.clone();
  thread::spawn(||{
    api::actor_api(core);
  });

  let handles = ActorMainHandles {
    lockscreen: lock_send,
    inhibitors: inh_send,
    inbox: core_recv
  };

  actor_main(handles);
}

fn actor_main(handles: ActorMainHandles) {
    let mut state = State {
        locked: false,
        inhibit_lid: false,
        locking: false,
        should_exit: false
    };
    handles.inhibitors.send(InhibitMessage::CreateDelay).unwrap();
    for message in handles.inbox {
      println!("Received message in core: {:?}", message);
      match message {
        CoreMessage::Lock => 
          if !(state.locked || state.locking) {
            handles.lockscreen.send(LockMessage::Lock).unwrap();
            state.locking = true;
          },
        CoreMessage::Unlock => 
          if state.locked && !state.locking {
            handles.lockscreen.send(LockMessage::Unlock).unwrap();
            state.locking = true;
          },
        CoreMessage::Locked => {
          state.locked = true;
          state.locking = false;
          handles.inhibitors.send(InhibitMessage::ReleaseDelay).unwrap();
        },
        CoreMessage::Unlocked => {
          state.locked = false;
          state.locking = false;
          if state.should_exit {
              std::process::exit(0);
          }
          handles.inhibitors.send(InhibitMessage::CreateDelay).unwrap();
        },
        CoreMessage::Exit => {
          if state.locked {
            handles.lockscreen.send(LockMessage::Unlock).unwrap();
          } else {
            std::process::exit(0);
          }
          state.should_exit = true;
        },
        CoreMessage::SuspendOnLid(value) => {
          if value == state.inhibit_lid {
            state.inhibit_lid = !value;
            if state.inhibit_lid {
                handles.inhibitors.send(InhibitMessage::CreateBlock).unwrap();
            } else {
                handles.inhibitors.send(InhibitMessage::ReleaseBlock).unwrap();
            }
          }
        },
        CoreMessage::Suspending => {
          if !state.locked {
            handles.lockscreen.send(LockMessage::Lock).unwrap();
            state.locking = true;
          }
        },
        CoreMessage::Suspended => {
          if !state.locked {
            handles.lockscreen.send(LockMessage::Lock).unwrap();
            state.locking = true;
          }
        },
        CoreMessage::QueryFlag(flag, channel) => {
          channel.send(match flag {
            CoreFlag::SuspendOnLid => !state.inhibit_lid
            //CoreFlag::Locking => state.locking,
            //CoreFlag::Locked => state.locked
          }).unwrap();
        }
      }
    }
}
