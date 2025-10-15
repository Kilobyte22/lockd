use std::mem;
use tokio::sync::{broadcast, mpsc, oneshot};
use zbus::{Connection, connection, interface};

use crate::{CurrentStatus, StateEvent, StateMessage};

pub async fn run(
    core_tx: mpsc::Sender<StateMessage>,
    mut event_rx: broadcast::Receiver<StateEvent>,
) -> anyhow::Result<()> {
    let connection = connection::Builder::session()?
        .name("de.kilobyte22.lockd")?
        .serve_at("/de/kilobyte22/lockd", Manager { core_tx })?
        .build()
        .await?;

    loop {
        let message = event_rx.recv().await?;
        let manager_ref = connection
            .object_server()
            .interface::<_, Manager>("/de/kilobyte22/lockd")
            .await?;
        let manager = manager_ref.get_mut().await;
        match message {
            StateEvent::Locking => {}
            StateEvent::Locked | StateEvent::Unlocked => {
                manager.locked_changed(manager_ref.signal_emitter()).await?;
            }
            StateEvent::Reload(_) => {}
            StateEvent::LidInhibitChanged(_) => {
                manager
                    .lid_switch_inhibited_changed(manager_ref.signal_emitter())
                    .await?;
            }
        }
    }
}

struct Manager {
    core_tx: mpsc::Sender<StateMessage>,
}

#[interface(name = "de.kilobyte22.lockd.Manager")]
impl Manager {
    async fn lock(&self, lockscreen_id: String) {
        self.core_tx
            .send(StateMessage::Lock { lockscreen_id })
            .await
            .expect("Cannot send message to state machine");
    }

    async fn reload(&self) {
        self.core_tx
            .send(StateMessage::Reload)
            .await
            .expect("Cannot send message to state machine");
    }

    #[zbus(property)]
    async fn locked(&self) -> bool {
        get_status(&self).await.current_lockscreen.is_some()
    }

    #[zbus(property)]
    async fn lid_switch_inhibited(&self) -> bool {
        get_status(&self).await.lid_switch_inhibited
    }

    #[zbus(property)]
    async fn set_lid_switch_inhibited(&self, value: bool) {
        self.core_tx
            .send(StateMessage::ChangeLidInhibit(value))
            .await
            .expect("Cannot send message to state machine");
    }
}

async fn get_status(manager: &Manager) -> CurrentStatus {
    let (tx, rx) = oneshot::channel();
    manager
        .core_tx
        .send(StateMessage::GetStatus { reply: tx })
        .await
        .expect("Cannot send message to state machine");
    rx.await.expect("Cannot receive message from state machine")
}
