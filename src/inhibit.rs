use dbus::{OwnedFd, Message, Connection, BusType};
use std::sync::{Mutex, Arc};
use std::sync::mpsc::{Sender, Receiver};
use msg::{InhibitMessage, CoreMessage};

pub fn actor_inhibit(core: Sender<CoreMessage>, cmd: Receiver<InhibitMessage>) {
  InhibitData {
    block: Arc::new(Mutex::new(None)),
    delay: Arc::new(Mutex::new(None)),
  }.actor_run(core, cmd);
}

struct InhibitData {
  block: Arc<Mutex<Option<OwnedFd>>>,
  delay: Arc<Mutex<Option<OwnedFd>>>
}

impl InhibitData {
  fn actor_run(&self, _core: Sender<CoreMessage>, cmd: Receiver<InhibitMessage>) {
    let connection = Connection::get_private(BusType::System).unwrap();
    for msg in cmd {
      match msg {
        InhibitMessage::CreateBlock => {
          let m = InhibitData::new_msg()
              .append3("handle-lid-switch", "lockd", "Lid Switch Disabled")
              .append1("block");
          let r = connection.send_with_reply_and_block(m, 2000).unwrap();
          let mut lock = self.block.lock().unwrap();
          *lock = Some(r.get1().unwrap());
        },
        InhibitMessage::CreateDelay => {
          let m = InhibitData::new_msg()
              .append3("sleep", "lockd", "lockd wants to put up lock screen before suspend")
              .append1("delay");
          let r = connection.send_with_reply_and_block(m, 2000).unwrap();
          let mut lock = self.delay.lock().unwrap();
          *lock = Some(r.get1().unwrap());
        },
        InhibitMessage::ReleaseBlock => {
          let mut lock = self.block.lock().unwrap();
          *lock = None; // lock gets dropped here
        },
        InhibitMessage::ReleaseDelay => {
          let mut lock = self.delay.lock().unwrap();
          *lock = None; // lock gets dropped here
        }
      }
    }
  }

  fn new_msg() -> Message {
    Message::new_method_call(
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
        "Inhibit").unwrap()
  }
}


