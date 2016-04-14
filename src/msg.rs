use std::fmt;
use std::sync::mpsc::Sender;

pub enum LockMessage {
  Lock,
  Unlock
}

pub enum InhibitMessage {
  CreateBlock,
  ReleaseBlock,
  CreateDelay,
  ReleaseDelay
}

//#[derive(Debug)]
pub enum CoreMessage {
  Lock,
  Unlock,
  Locked,
  Unlocked,
  // ReloadConfig,
  Exit,
  SuspendOnLid(bool),
  Suspending,
  Suspended,
  QueryFlag(CoreFlag, Sender<bool>),
  AutoLock,
  SetAutoLock(bool)
}

impl fmt::Debug for CoreMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CoreMessage::QueryFlag(ref flag, _) => {
                write!(f, "QueryFlag({:?})", flag)
            },
            ref otherwise => otherwise.fmt(f)
        }
    }
}

#[derive(Debug)]
pub enum CoreFlag {
  SuspendOnLid,
  //Locking, TODO: Add dbus api
  //Locked,
  AutoLock
}
