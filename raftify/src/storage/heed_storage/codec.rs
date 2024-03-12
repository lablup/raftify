use heed_traits::{BoxedError, BytesDecode, BytesEncode};
use prost::Message;
use std::{borrow::Cow, fmt::Write};

use super::constant::ENTRY_KEY_LENGTH;
use crate::raft::eraftpb::Entry;

pub fn format_entry_key_string(entry_key: &str) -> String {
    let entry_key: u64 = entry_key.parse().unwrap();

    let mut result = String::new();
    write!(result, "{:0width$}", entry_key, width = ENTRY_KEY_LENGTH).unwrap();
    result
}

#[derive(Eq, PartialEq)]
pub struct HeedEntryKeyString(String);

impl<'a> BytesEncode<'a> for HeedEntryKeyString {
    type EItem = String;

    fn bytes_encode(item: &'a Self::EItem) -> std::result::Result<Cow<'a, [u8]>, BoxedError> {
        Ok(Cow::Owned(
            format_entry_key_string(item).as_bytes().to_vec(),
        ))
    }
}

impl<'a> BytesDecode<'a> for HeedEntryKeyString {
    type DItem = String;

    fn bytes_decode(bytes: &'a [u8]) -> std::result::Result<Self::DItem, BoxedError> {
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

pub enum HeedEntry {}

impl BytesEncode<'_> for HeedEntry {
    type EItem = Entry;

    fn bytes_encode(item: &Self::EItem) -> std::result::Result<Cow<'_, [u8]>, BoxedError> {
        let mut bytes = vec![];
        item.encode(&mut bytes)?;
        Ok(Cow::Owned(bytes))
    }
}

impl BytesDecode<'_> for HeedEntry {
    type DItem = Entry;

    fn bytes_decode(bytes: &[u8]) -> std::result::Result<Self::DItem, BoxedError> {
        Ok(Entry::decode(bytes)?)
    }
}
