use std::mem;
use tokio::sync::{broadcast, mpsc, oneshot};
use zbus::{Connection, connection, interface};

use crate::{StateEvent, StateMessage};

pub async fn start(core_tx: mpsc::Sender<StateMessage>) -> anyhow::Result<()> {
    mem::forget(
        connection::Builder::session()?
            .name("de.kilobyte22.lockd")?
            .serve_at("/de/kilobyte22/lockd", Manager { core_tx })?
            .build()
            .await?,
    );

    Ok(())
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
        let (tx, rx) = oneshot::channel();
        self.core_tx
            .send(StateMessage::GetStatus { reply: tx })
            .await
            .expect("Cannot send message to state machine");
        let status = rx.await.expect("Cannot receive message from state machine");

        status.current_lockscreen.is_some()
    }
}
