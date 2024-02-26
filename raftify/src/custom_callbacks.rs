use lazy_static::lazy_static;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::Mutex;

use crate::Peers;

type ON_MEMBER_CHANGED_CALLBACK = dyn FnMut(Arc<Mutex<Peers>>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

lazy_static! {
    pub static ref ON_MEMBER_CHANGED: Arc<Mutex<Option<Box<ON_MEMBER_CHANGED_CALLBACK>>>> = Arc::new(Mutex::new(None));
}

pub async fn set_on_member_changed(callback: Box<ON_MEMBER_CHANGED_CALLBACK>) {
    let mut global_callback = ON_MEMBER_CHANGED.lock().await;
    *global_callback = Some(callback);
}
