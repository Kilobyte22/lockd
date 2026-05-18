extern crate alloc;

use crate::config::{Config, Lockscreen};
use crate::event::EventReceiver;
use anyhow::Context;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fmt, process};
use std::os::fd::AsRawFd;
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::{net, signal, task};
use tokio::io::AsyncReadExt;

mod config;
mod dbus;
mod event;
mod util;
mod lid;

enum StateMessage {
    /// lockd supports having multiple lockscreens for different purposes. The [[lockscreen_id]] specifies
    /// which one is used, as defined in the configuration
    Lock {
        lockscreen_id: String,
    },
    Unlock,
    UnlockedByUser,
    Reload,
    GetStatus {
        reply: oneshot::Sender<CurrentStatus>,
    },
    ChangeLidInhibit(bool),
    LockscreenReady,
}

impl fmt::Debug for StateMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateMessage::Lock { lockscreen_id } => f
                .debug_struct("StateMessage::Lock")
                .field("lockscreen_id", lockscreen_id)
                .finish(),
            StateMessage::Unlock => f.debug_struct("StateMessage::Unlock").finish(),
            StateMessage::UnlockedByUser => f.debug_struct("StateMessage::UnlockedByUser").finish(),
            StateMessage::Reload => f.debug_struct("StateMessage::Reload").finish(),
            StateMessage::GetStatus { reply: _ } => f
                .debug_struct("StateMessage::GetStatus")
                .finish_non_exhaustive(),
            StateMessage::ChangeLidInhibit(value) => f
                .debug_tuple("StateMessage::ChangeLidInhibit")
                .field(value)
                .finish(),
            StateMessage::LockscreenReady => f.debug_struct("StateMessage::LockscreenReady").finish(),
        }
    }
}

#[derive(Debug, Clone)]
enum StateEvent {
    Locking,
    Locked,
    Unlocked,
    Reload(Arc<Config>),
    LidInhibitChanged(bool),
}

#[derive(Debug, Clone)]
enum State {
    Unlocked,
    Locking(LockscreenInfo),
    Locked(LockscreenInfo),
}

impl State {
    fn get_lockscreen_info(&self) -> Option<&LockscreenInfo> {
        match self {
            State::Unlocked => None,
            State::Locking(lockscreen_info) => Some(lockscreen_info),
            State::Locked(lockscreen_info) => Some(lockscreen_info),
        }
    }
}

#[derive(Debug, Clone)]
struct LockscreenInfo {
    pid: u32,
    name: String,
    config: Lockscreen,
}

struct CurrentStatus {
    current_lockscreen: Option<String>,
    lid_switch_inhibited: bool,
}

async fn state_machine(
    mut config: config::ConfigBundle,
    mut inbox: mpsc::Receiver<StateMessage>,
    tx: mpsc::Sender<StateMessage>,
    event_tx: broadcast::Sender<StateEvent>,
) {
    let mut state = State::Unlocked;
    let mut lid_switch_inhibited = false;
    loop {
        let msg = inbox.recv().await.expect("Inbox channel closed");
        tracing::debug!("State Message: {msg:?}");
        match msg {
            StateMessage::Lock { lockscreen_id } => {
                if matches!(state, State::Locked(_)) {
                    // We are already locked, we shouldn't lock a second time
                    continue;
                }
                if let Some(lockscreen) = config.lockscreens.get(&lockscreen_id) {
                    let mut command = Command::new(&lockscreen.command[0]);
                    command.args(&lockscreen.command[1..]);

                    let ready_rx = if lockscreen.ready_fd {
                        let (send, receive) = net::unix::pipe::pipe().expect("failed to create pipe");
                        unsafe {
                            command.pre_exec(move || {
                                std::env::set_var("READYFD", format!("{}", send.as_raw_fd()));
                                Ok(())
                            });
                        }
                        Some(receive)
                    } else {
                        None
                    };

                    let child = command
                        .spawn()
                        .unwrap();
                    
                    let lockscreen_info = LockscreenInfo {
                        pid: child.id().expect("Child has no pid"),
                        name: lockscreen_id.clone(),
                        config: lockscreen.clone(),
                    };
                    tracing::debug!("Lockscreen active, PID: {:?}", child.id());
                    task::spawn(watch_child(child, tx.clone()));
                    
                    // TODO: Kommentieren was zum fick hier passiert
                    if let Some(mut ready_rx) = ready_rx {
                        event_tx.send(StateEvent::Locking).unwrap();
                        state = State::Locking(lockscreen_info);
                        let tx = tx.clone();
                        task::spawn(async move {
                            let mut buf = vec![0];
                            match ready_rx.read(&mut buf).await {
                                Ok(_) => tx.send(StateMessage::LockscreenReady).await.unwrap(),
                                Err(_) => {},
                            }
                            
                        });
                    } else {
                        event_tx.send(StateEvent::Locked).unwrap();
                        state = State::Locked(lockscreen_info);
                    }
                }
            }
            StateMessage::Unlock => {
                match state.get_lockscreen_info() {
                    None => {}
                    Some(LockscreenInfo { config: ls_config, pid: child, .. }) => {
                        if ls_config.can_kill {
                            util::kill_process(*child).unwrap();
                        } else {
                            tracing::warn!("Lockscreen does not support unlocking from the outside");
                        }
                    }
                }
            },
            StateMessage::UnlockedByUser => {
                state = State::Unlocked;
                event_tx.send(StateEvent::Unlocked).unwrap();
            }
            StateMessage::Reload => match config.reload().await {
                Ok(()) => {
                    event_tx
                        .send(StateEvent::Reload(config.config.clone()))
                        .unwrap();
                }
                Err(_) => {}
            },
            StateMessage::GetStatus { reply } => {
                let _ = reply.send(CurrentStatus {
                    current_lockscreen: match &state {
                        State::Locked(LockscreenInfo { name, .. }) => Some(name.to_owned()),
                        _ => None,
                    },
                    lid_switch_inhibited,
                });
            }
            StateMessage::ChangeLidInhibit(value) => {
                if lid_switch_inhibited != value {
                    lid_switch_inhibited = value;
                    event_tx.send(StateEvent::LidInhibitChanged(value)).unwrap();
                }
            }
            StateMessage::LockscreenReady => {
                todo!()
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let xdg_config_home =
        PathBuf::from(std::env::var("XDG_CONFIG_HOME").expect("XDG_CONFIG_HOME not set"));
    let config_path = xdg_config_home.join("lockd").join("lockd.toml");
    let config = config::ConfigBundle::load(config_path)
        .await
        .context("failed to load configuration")?;

    let (event_tx, event_rx) = broadcast::channel::<StateEvent>(100);
    let (tx, rx) = mpsc::channel(100);
    let receiver = EventReceiver::new(event_tx.clone());

    dbus::start(
        config.config.clone(),
        receiver,
        tx.clone(),
    ).await;

    task::spawn(handle_signals(tx.clone()));

    {
        let lockscreen_id = config.default_lockscreen.clone();
        let tx = tx.clone();
        task::spawn(async move {
            match lid::watch(tx, lockscreen_id, event_rx).await {
                Ok(()) => {}
                Err(e) => {
                    tracing::warn!("failed to watch lockscreen, lid locking will only be available when suspend is active: {}", e);
                }
            }
        });
    }

    state_machine(config, rx, tx, event_tx).await;

    Ok(())
}

async fn watch_child(mut child: Child, tx: mpsc::Sender<StateMessage>) {
    child.wait().await.unwrap();
    tx.send(StateMessage::UnlockedByUser).await.unwrap();
}

async fn error_log_wrapper<F: Future<Output = anyhow::Result<()>> + Send + 'static>(
    future: F,
) -> () {
    match future.await {
        Ok(()) => {}
        Err(e) => {
            tracing::error!("{}", e);
            tracing::debug!("Details: {:#?}", e);
            process::exit(1);
        }
    }
}

async fn handle_signals(tx: mpsc::Sender<StateMessage>) {
    let mut handler = signal::unix::signal(signal::unix::SignalKind::hangup()).unwrap();
    loop {
        handler.recv().await;
        tx.try_send(StateMessage::Reload).unwrap();
    }
}
