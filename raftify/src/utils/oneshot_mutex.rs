use tokio::sync::Mutex;

pub struct OneShotMutex<T> {
    data: Mutex<Option<T>>,
}

impl<T> OneShotMutex<T> {
    pub fn new(data: T) -> Self {
        OneShotMutex {
            data: Mutex::new(Some(data)),
        }
    }

    pub async fn lock(&self) -> Option<T> {
        let mut data = self.data.lock().await;
        data.take()
    }
}
