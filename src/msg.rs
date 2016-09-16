use std::fmt;
use std::sync::mpsc::Sender;

pub enum LockMessage {
  Lock,
  Unlock,
  SetLockscreen(String, Vec<String>)
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
  ReloadConfig,
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
            CoreMessage::Lock => write!(f, "Lock"),
            CoreMessage::Unlock => write!(f, "Unlock"),
            CoreMessage::Locked => write!(f, "Locked"),
            CoreMessage::Unlocked => write!(f, "Unlocked"),
            CoreMessage::Exit => write!(f, "Exit"),
            CoreMessage::AutoLock => write!(f, "AutoLock"),
            CoreMessage::Suspending => write!(f, "Suspending"),
            CoreMessage::Suspended => write!(f, "Suspended"),
            CoreMessage::ReloadConfig => write!(f, "ReloadConfig"),

            CoreMessage::SuspendOnLid(flag) => {
                write!(f, "SuspendOnLid({:?})", flag)
            },
            CoreMessage::QueryFlag(ref flag, _) => {
                write!(f, "QueryFlag({:?})", flag)
            },
            CoreMessage::SetAutoLock(flag) => {
                write!(f, "SetAutoLock({:?})", flag)
            },
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
