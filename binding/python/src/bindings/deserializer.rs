use std::sync::Mutex;

use once_cell::sync::Lazy;
use prost::Message as PMessage;
use pyo3::{prelude::*, types::PyBytes, PyObject, Python};
use raftify::raft::{
    derializer::{format_confchange, format_confchangev2, Bytes, CustomDeserializer},
    eraftpb::{ConfChange, ConfChangeV2},
};

pub struct PythonDeserializer;

// TODO: Refactor below codes to reduce code redundancy.
static ENTRY_CONTEXT_DESERIALIZE_CB: Lazy<Mutex<Option<PyObject>>> = Lazy::new(|| Mutex::new(None));
static ENTRY_DATA_DESERIALIZE_CB: Lazy<Mutex<Option<PyObject>>> = Lazy::new(|| Mutex::new(None));
static CONFCHANGEV2_CONTEXT_DESERIALIZE_CB: Lazy<Mutex<Option<PyObject>>> =
    Lazy::new(|| Mutex::new(None));
static CONFCHANGE_CONTEXT_DESERIALIZE_CB: Lazy<Mutex<Option<PyObject>>> =
    Lazy::new(|| Mutex::new(None));
static MESSAGE_CONTEXT_DESERIALIZER_CB: Lazy<Mutex<Option<PyObject>>> =
    Lazy::new(|| Mutex::new(None));
static SNAPSHOT_DATA_DESERIALIZER_CB: Lazy<Mutex<Option<PyObject>>> =
    Lazy::new(|| Mutex::new(None));

#[pyfunction]
pub fn set_entry_context_deserializer(cb: PyObject) {
    *ENTRY_CONTEXT_DESERIALIZE_CB.lock().unwrap() = Some(cb);
}

#[pyfunction]
pub fn set_entry_data_deserializer(cb: PyObject) {
    *ENTRY_DATA_DESERIALIZE_CB.lock().unwrap() = Some(cb);
}

#[pyfunction]
pub fn set_confchangev2_context_deserializer(cb: PyObject) {
    *CONFCHANGEV2_CONTEXT_DESERIALIZE_CB.lock().unwrap() = Some(cb);
}

#[pyfunction]
pub fn set_confchange_context_deserializer(cb: PyObject) {
    *CONFCHANGE_CONTEXT_DESERIALIZE_CB.lock().unwrap() = Some(cb);
}

#[pyfunction]
pub fn set_message_context_deserializer(cb: PyObject) {
    *MESSAGE_CONTEXT_DESERIALIZER_CB.lock().unwrap() = Some(cb);
}

#[pyfunction]
pub fn set_snapshot_data_deserializer(cb: PyObject) {
    *SNAPSHOT_DATA_DESERIALIZER_CB.lock().unwrap() = Some(cb);
}

impl CustomDeserializer for PythonDeserializer {
    fn entry_context_deserialize(&self, v: &Bytes) -> String {
        fn deserialize(py: Python, data: &[u8]) -> String {
            let callback_lock = ENTRY_CONTEXT_DESERIALIZE_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                callback
                    .call(py, (PyBytes::new(py, data),), None)
                    .unwrap()
                    .as_ref(py)
                    .to_string()
            } else {
                format!("{:?}", data)
            }
        }

        Python::with_gil(|py| match v {
            Bytes::Prost(v) => format!("{:?}", deserialize(py, &v[..])),
            Bytes::Protobuf(v) => format!("{:?}", deserialize(py, &v[..])),
        })
    }

    fn entry_data_deserialize(&self, v: &Bytes) -> String {
        fn deserialize(py: Python, data: &[u8]) -> String {
            let callback_lock = ENTRY_DATA_DESERIALIZE_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                if data.len() != 0 {
                    let res = callback.call(py, (PyBytes::new(py, data),), None).unwrap();
                    let res = res.as_ref(py);

                    if !res.is_none() {
                        return res.to_string();
                    }

                    if let Ok(cc) = ConfChange::decode(data) {
                        return format_confchange(&cc);
                    }
                    if let Ok(cc) = ConfChangeV2::decode(data) {
                        return format_confchangev2(&cc);
                    }
                }
            }

            format!("{:?}", data)
        }

        Python::with_gil(|py| match v {
            Bytes::Prost(v) => format!("{:?}", deserialize(py, &v[..])),
            Bytes::Protobuf(v) => format!("{:?}", deserialize(py, &v[..])),
        })
    }

    fn confchangev2_context_deserialize(&self, v: &Bytes) -> String {
        fn deserialize(py: Python, data: &[u8]) -> String {
            let callback_lock = CONFCHANGEV2_CONTEXT_DESERIALIZE_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                callback
                    .call(py, (PyBytes::new(py, data),), None)
                    .unwrap()
                    .as_ref(py)
                    .to_string()
            } else {
                format!("{:?}", data)
            }
        }

        Python::with_gil(|py| match v {
            Bytes::Prost(v) => format!("{:?}", deserialize(py, &v[..])),
            Bytes::Protobuf(v) => format!("{:?}", deserialize(py, &v[..])),
        })
    }

    fn confchange_context_deserialize(&self, v: &Bytes) -> String {
        fn deserialize(py: Python, data: &[u8]) -> String {
            let callback_lock = CONFCHANGE_CONTEXT_DESERIALIZE_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                callback
                    .call(py, (PyBytes::new(py, data),), None)
                    .unwrap()
                    .as_ref(py)
                    .to_string()
            } else {
                format!("{:?}", data)
            }
        }

        Python::with_gil(|py| match v {
            Bytes::Prost(v) => format!("{:?}", deserialize(py, &v[..])),
            Bytes::Protobuf(v) => format!("{:?}", deserialize(py, &v[..])),
        })
    }

    fn message_context_deserializer(&self, v: &Bytes) -> String {
        fn deserialize(py: Python, data: &[u8]) -> String {
            let callback_lock = MESSAGE_CONTEXT_DESERIALIZER_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                callback
                    .call(py, (PyBytes::new(py, data),), None)
                    .unwrap()
                    .as_ref(py)
                    .to_string()
            } else {
                format!("{:?}", data)
            }
        }

        Python::with_gil(|py| match v {
            Bytes::Prost(v) => format!("{:?}", deserialize(py, &v[..])),
            Bytes::Protobuf(v) => format!("{:?}", deserialize(py, &v[..])),
        })
    }

    fn snapshot_data_deserializer(&self, v: &Bytes) -> String {
        fn deserialize(py: Python, data: &[u8]) -> String {
            let callback_lock = SNAPSHOT_DATA_DESERIALIZER_CB.lock().unwrap();

            if let Some(callback) = &*callback_lock {
                callback
                    .call(py, (PyBytes::new(py, data),), None)
                    .unwrap()
                    .as_ref(py)
                    .to_string()
            } else {
                format!("{:?}", data)
            }
        }

        Python::with_gil(|py| match v {
            Bytes::Prost(v) => format!("{:?}", deserialize(py, &v[..])),
            Bytes::Protobuf(v) => format!("{:?}", deserialize(py, &v[..])),
        })
    }
}
