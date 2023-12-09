use raft::derializer::{Bytes, CustomDeserializer};
pub struct BincodeDeserializer;

impl CustomDeserializer for BincodeDeserializer {
    fn entry_context_deserialize(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }

    fn entry_data_deserialize(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }

    fn confchangev2_context_deserialize(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }

    fn confchange_context_deserialize(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }

    fn message_context_deserializer(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }

    fn snapshot_data_deserializer(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }
}
