use tokio::sync::Mutex;

use crate::raft::eraftpb::{ConfChange, ConfChangeSingle, ConfChangeV2};

pub fn to_confchange_v2(conf_change: ConfChange) -> ConfChangeV2 {
    let mut cc_v2 = ConfChangeV2::default();

    let mut cs = ConfChangeSingle::default();
    cs.set_node_id(conf_change.node_id);
    cs.set_change_type(conf_change.get_change_type());
    cc_v2.set_changes(vec![cs]);
    cc_v2.set_context(conf_change.context);

    cc_v2
}

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

pub mod macro_utils {
    macro_rules! function_name {
        () => {{
            fn f() {}
            fn type_name_of<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            let name = type_name_of(f);
            &name[..name.len() - 3] // subtract length of "f"
        }};
    }

    pub(crate) use function_name;
}
