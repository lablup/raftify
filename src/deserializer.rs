use raft::{
    derializer::{format_confchangev2, Bytes, CustomDeserializer},
    eraftpb::ConfChangeV2,
};
pub struct MyDeserializer;
use prost::Message as PMessage;

impl CustomDeserializer for MyDeserializer {
    fn entry_context_deserialize(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", (&v[..])),
            Bytes::Protobuf(v) => format!("{:?}", v.as_ref()),
        }
    }

    fn entry_data_deserialize(&self, v: &Bytes) -> String {
        match v {
            Bytes::Prost(v) => format!("{:?}", {
                if let Ok(cc) = ConfChangeV2::decode(&v[..]) {
                    return format_confchangev2(&cc);
                }

                &v[..]
            }),
            Bytes::Protobuf(v) => format!("{:?}", {
                if let Ok(cc) = ConfChangeV2::decode(v.as_ref()) {
                    return format_confchangev2(&cc);
                }

                v.as_ref()
            }),
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
