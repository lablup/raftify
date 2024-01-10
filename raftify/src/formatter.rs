use bincode::deserialize;
use prost::Message as PMessage;
use std::{fmt::Debug, marker::PhantomData, net::SocketAddr};

use super::{AbstractLogEntry, AbstractStateMachine};
use crate::raft::{
    eraftpb::{ConfChange, ConfChangeV2},
    formatter::{
        format_confchange, format_confchangev2, Bytes, CustomFormatter as CustomFormatter_,
    },
};

pub struct CustomFormatter<
    LogEntry: AbstractLogEntry + Debug + 'static,
    FSM: AbstractStateMachine + Debug + Clone + Send + Sync + 'static,
> {
    _marker: PhantomData<LogEntry>,
    _marker2: PhantomData<FSM>,
}

impl<
        LogEntry: AbstractLogEntry + Debug,
        FSM: AbstractStateMachine + Debug + Clone + Send + Sync + 'static,
    > CustomFormatter<LogEntry, FSM>
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            _marker2: PhantomData,
        }
    }
}

impl<
        LogEntry: AbstractLogEntry + Debug,
        FSM: AbstractStateMachine + Debug + Clone + Send + Sync + 'static,
    > Default for CustomFormatter<LogEntry, FSM>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
        LogEntry: AbstractLogEntry + Debug,
        FSM: AbstractStateMachine + Debug + Clone + Send + Sync + 'static,
    > CustomFormatter_ for CustomFormatter<LogEntry, FSM>
{
    fn format_entry_context(&self, v: &Bytes) -> String {
        let v = match v {
            Bytes::Prost(v) => v,
            Bytes::Protobuf(v) => v.as_ref(),
        };

        if let Ok(v) = deserialize::<u64>(v) {
            return format!("{:?}", v);
        }

        format!("{:?}", v)
    }

    fn format_entry_data(&self, v: &Bytes) -> String {
        let v = match v {
            Bytes::Prost(v) => v,
            Bytes::Protobuf(v) => v.as_ref(),
        };

        // Empty byte slice is being translated to empty confchange v1 whose all values are 0,
        // but we won't need such thing anyway.
        if !v.is_empty() {
            if let Ok(cc) = ConfChange::decode(v) {
                return format_confchange(&cc);
            }
            if let Ok(cc) = ConfChangeV2::decode(v) {
                return format_confchangev2(&cc);
            }
        }

        if let Ok(log_entry) = LogEntry::decode(v) {
            return format!("{:?}", log_entry);
        }

        format!("{:?}", v)
    }

    fn format_confchangev2_context(&self, v: &Bytes) -> String {
        let v = match v {
            Bytes::Prost(v) => v,
            Bytes::Protobuf(v) => v.as_ref(),
        };

        if let Ok(addrs) = deserialize::<Vec<SocketAddr>>(v) {
            return format!("{:?}", addrs);
        }

        format!("{:?}", v)
    }

    fn format_confchange_context(&self, v: &Bytes) -> String {
        let v = match v {
            Bytes::Prost(v) => v,
            Bytes::Protobuf(v) => v.as_ref(),
        };

        if let Ok(addrs) = deserialize::<Vec<SocketAddr>>(v) {
            return format!("{:?}", addrs);
        }

        format!("{:?}", v)
    }

    fn format_message_context(&self, v: &Bytes) -> String {
        let v = match v {
            Bytes::Prost(v) => v,
            Bytes::Protobuf(v) => v.as_ref(),
        };
        format!("{:?}", v)
    }

    fn format_snapshot_data(&self, v: &Bytes) -> String {
        let v = match v {
            Bytes::Prost(v) => v,
            Bytes::Protobuf(v) => v.as_ref(),
        };

        if let Ok(fsm) = FSM::decode(v) {
            return format!("{:?}", fsm);
        }

        format!("{:?}", v)
    }
}
