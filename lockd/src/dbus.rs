use crate::config::{Config, ConfigBundle};
use crate::{StateEvent, StateMessage};
use futures::StreamExt;
use lockd_common::dbus::{ManagerProxy, SessionProxy};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use zbus::Connection;

#[derive(Debug)]
pub enum DBusMessage {
    Locked,
    Unlocked,
}

pub async fn run(
    config: Arc<ConfigBundle>,
    mut event_rx: broadcast::Receiver<StateEvent>,
    core_tx: mpsc::Sender<StateMessage>,
) -> anyhow::Result<()> {
    let connection = Connection::system().await?;

    let manager = lockd_common::dbus::ManagerProxy::new(&connection).await?;
    let session_path = manager.get_session("self").await?;

    tracing::debug!("session path: {:?}", session_path);

    let session = lockd_common::dbus::SessionProxy::builder(&connection)
        .path(session_path)?
        .build()
        .await?;

    let mut handlers_task = task::spawn(crate::error_log_wrapper(run_handlers(
        Arc::new(config.config.clone()),
        session.clone(),
        manager.clone(),
        core_tx.clone(),
    )));

    let mut inhibit_fd = Some(
        manager
            .inhibit("sleep", "lockd", "Needs to lock screen", "delay")
            .await?,
    );
    tracing::trace!("Grabbing suspend inhibitor");
    loop {
        match event_rx.recv().await? {
            StateEvent::Locked => {
                session.set_locked_hint(true).await?;
                inhibit_fd.take();
                tracing::trace!("Releasing suspend inhibitor");
            }
            StateEvent::Unlocked => {
                session.set_locked_hint(false).await?;
                inhibit_fd = Some(
                    manager
                        .inhibit("sleep", "lockd", "Needs to lock screen", "delay")
                        .await?,
                );
                tracing::trace!("Grabbing suspend inhibitor");
            }
            StateEvent::Locking => {}
        }
    }
}

async fn handle_lock_messages(
    mut stream: lockd_common::dbus::LockStream,
    tx: mpsc::Sender<StateMessage>,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    while let Some(_) = stream.next().await {
        tx.send(StateMessage::Lock {
            lockscreen_id: config.default_lockscreen.clone(),
        })
        .await?;
    }
    Ok(())
}

async fn handle_unlock_messages(
    mut stream: lockd_common::dbus::UnlockStream,
    tx: mpsc::Sender<StateMessage>,
) -> anyhow::Result<()> {
    while let Some(_) = stream.next().await {
        tx.send(StateMessage::Unlock).await?;
    }
    Ok(())
}

async fn handle_pre_sleep_messages(
    mut stream: lockd_common::dbus::PrepareForSleepStream,
    tx: mpsc::Sender<StateMessage>,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    while let Some(_) = stream.next().await {
        tx.send(StateMessage::Lock {
            lockscreen_id: config.default_lockscreen.clone(),
        })
        .await?;
    }
    Ok(())
}

async fn run_handlers(
    config: Arc<Config>,
    session: SessionProxy<'static>,
    manager: ManagerProxy<'static>,
    core_tx: mpsc::Sender<StateMessage>,
) -> anyhow::Result<()> {
    let ((), (), ()) = tokio::try_join!(
        handle_lock_messages(
            session.receive_lock().await?,
            core_tx.clone(),
            config.clone()
        ),
        handle_unlock_messages(session.receive_unlock().await?, core_tx.clone()),
        handle_pre_sleep_messages(manager.receive_prepare_for_sleep().await?, core_tx, config),
    )?;
    Ok(())
}
