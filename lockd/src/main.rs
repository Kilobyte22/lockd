extern crate alloc;

use crate::config::{Config, Lockscreen};
use crate::dbus::DBusMessage;
use anyhow::Context;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc};
use tokio::{signal, task};

mod config;
mod dbus;
mod util;

#[derive(Debug)]
enum StateMessage {
    /// lockd supports having multiple lockscreens for different purposes. The [[lockscreen_id]] specifies
    /// which one is used, as defined in the configuration
    Lock {
        lockscreen_id: String,
    },
    Unlock,
    UnlockedByUser,
    Reload,
}

#[derive(Debug, Clone)]
enum StateEvent {
    Locking,
    Locked,
    Unlocked,
    Reload(Arc<Config>),
}

enum State {
    Unlocked,
    Locking,
    Locked,
}

async fn state_machine(
    mut config: config::ConfigBundle,
    mut inbox: mpsc::Receiver<StateMessage>,
    tx: mpsc::Sender<StateMessage>,
    event_tx: broadcast::Sender<StateEvent>,
) {
    let mut running_lockscreen: Option<(u32, Lockscreen)> = None;
    loop {
        let msg = inbox.recv().await.expect("Inbox channel closed");
        tracing::debug!("State Message: {msg:?}");
        match msg {
            StateMessage::Lock { lockscreen_id } => {
                if running_lockscreen.is_some() {
                    // We are already locked, we shouldn't lock a second time
                    continue;
                }
                if let Some(lockscreen) = config.lockscreens.get(&lockscreen_id) {
                    let child = Command::new(&lockscreen.command[0])
                        .args(&lockscreen.command[1..])
                        .spawn()
                        .unwrap();
                    running_lockscreen =
                        Some((child.id().expect("Child has no pid"), lockscreen.clone()));
                    tracing::debug!("Lockscreen active, PID: {:?}", child.id());
                    task::spawn(watch_child(child, tx.clone()));
                    event_tx.send(StateEvent::Locked).unwrap();
                }
            }
            StateMessage::Unlock => match running_lockscreen.take() {
                None => {}
                Some((child, ls_config)) => {
                    if ls_config.can_kill {
                        util::kill_process(child).unwrap();
                    } else {
                        tracing::warn!("Lockscreen does not support unlocking from the outside");
                    }
                }
            },
            StateMessage::UnlockedByUser => {
                running_lockscreen = None;
                event_tx.send(StateEvent::Unlocked).unwrap();
            }
            StateMessage::Reload => {
                match config.reload().await {
                    Ok(()) => {
                        event_tx.send(StateEvent::Reload(config.config.clone())).unwrap();
                    }
                    Err(_) => {}
                }
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
    let config = 
        config::ConfigBundle::load(config_path)
            .await
            .context("failed to load configuration")?;

    let (event_tx, event_rx) = broadcast::channel::<StateEvent>(100);
    let (tx, rx) = mpsc::channel(100);
    task::spawn(error_log_wrapper(dbus::run(
        config.config.clone(),
        event_rx,
        tx.clone(),
    )));
    
    task::spawn(handle_signals(tx.clone()));
    
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