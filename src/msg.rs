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
  LidClosed,
  Suspending,
  Suspended
}
