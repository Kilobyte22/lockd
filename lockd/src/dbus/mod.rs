use crate::config::Config;
use crate::{StateEvent, StateMessage};
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task;

mod lockd;
mod logind;

pub async fn run(
    config: Arc<Config>,
    event_rx: broadcast::Receiver<StateEvent>,
    core_tx: mpsc::Sender<StateMessage>,
) -> anyhow::Result<()> {
    lockd::start(core_tx.clone()).await?;
    logind::run(config, event_rx, core_tx).await?;
    Ok(())
}
