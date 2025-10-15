use tokio::sync::broadcast;

// Wrapper around an [[broadcast::Sender]] to prevent doing anything except for creating receivers with it
#[derive(Clone)]
pub struct EventReceiver<T> {
    tx: broadcast::Sender<T>,
}

impl<T> EventReceiver<T> {
    pub fn new(tx: broadcast::Sender<T>) -> Self {
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }
}
