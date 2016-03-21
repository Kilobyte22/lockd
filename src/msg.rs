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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum CoreFlag {
  SuspendOnLid,
  //Locking, TODO: Add dbus api
  //Locked,
  AutoLock
}
