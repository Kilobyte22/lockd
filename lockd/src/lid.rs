use std::time;
use tokio::fs;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::error::Elapsed;
use crate::{StateEvent, StateMessage};

pub async fn watch(tx: mpsc::Sender<StateMessage>, mut lockscreen_id: String, mut event_rx: broadcast::Receiver<StateEvent>) -> anyhow::Result<()> {
    let dir_entry = fs::read_dir("/proc/acpi/button/lid").await?.next_entry().await?.ok_or_else(|| anyhow::anyhow!("No lid switch found"))?;
    let file = dir_entry.path().join("state");
    let mut last_closed = None;
    // For now we only implement polling. In the future other options may be available
    loop {
        let data = fs::read_to_string(&file).await?;
        let mut data_iter = data.split(":");
        let (key, value, tail) = (data_iter.next().map(str::trim), data_iter.next().map(str::trim), data_iter.next());
        match (key, value, tail) {
            (Some("state"), Some("closed"), None) => {
                if last_closed == Some(false) {
                    tx.send(StateMessage::Lock {
                        lockscreen_id: lockscreen_id.clone(),
                    }).await?;
                    last_closed = Some(true);
                }
            }
            (Some("state"), Some("open"), None) => {
                last_closed = Some(false);
            },
            _ => {
                tracing::warn!("failed to parse lid switch state: {data}");
            }
        }
        
        match tokio::time::timeout(time::Duration::from_millis(500), event_rx.recv()).await {
            Ok(msg) => {
                match msg? {
                    StateEvent::Locking => {}
                    StateEvent::Locked => {}
                    StateEvent::Unlocked => {}
                    StateEvent::Reload(config) => {
                        lockscreen_id = config.default_lockscreen.clone();
                    }
                    StateEvent::LidInhibitChanged(_) => {}
                }
            }
            Err(_) => {}
        }
    }
}