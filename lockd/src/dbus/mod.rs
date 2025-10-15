use crate::config::Config;
use crate::event::EventReceiver;
use crate::{StateEvent, StateMessage};
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task;

mod lockd;
mod logind;

pub async fn start(
    config: Arc<Config>,
    event_rx: EventReceiver<StateEvent>,
    core_tx: mpsc::Sender<StateMessage>,
) {
    task::spawn(crate::error_log_wrapper(lockd::run(core_tx.clone(), event_rx.subscribe())));
    task::spawn(crate::error_log_wrapper(logind::run(config, event_rx.subscribe(), core_tx)));
}
