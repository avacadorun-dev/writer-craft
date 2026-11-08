//! Duplicate-key checking leaves arbitrary-precision source values untouched.
use std::collections::HashSet;
use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use super::{Result, StudioError};

pub(super) fn reject_duplicate_keys(bytes: &[u8]) -> Result<()> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    UniqueJson::deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(())
}

struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        decoder.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueJson;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_string<E: serde::de::Error>(self, _: String) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(UniqueJson)
    }
    fn visit_some<D: Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        UniqueJson::deserialize(decoder)
    }
    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut sequence: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        while sequence.next_element::<UniqueJson>()?.is_some() {}
        Ok(UniqueJson)
    }
    fn visit_map<A: MapAccess<'de>>(
        self,
        mut object: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut keys = HashSet::new();
        while let Some(key) = object.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(serde::de::Error::custom("duplicate JSON object key"));
            }
            object.next_value::<UniqueJson>()?;
        }
        Ok(UniqueJson)
    }
}

pub(super) fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(StudioError::Validation(format!(
        "Invalid project transfer: {}",
        message.into()
    )))
}

/// Root framing is data too: RawValue intentionally omits its surrounding whitespace.
pub(super) fn encode_preserving_manifest(
    original: &str,
    current: &serde_json::Value,
) -> Result<Vec<u8>> {
    let previous: serde_json::Value = serde_json::from_str(original)?;
    if previous == *current {
        return Ok(original.as_bytes().to_vec());
    }
    let raw: Box<serde_json::value::RawValue> = serde_json::from_str(original)?;
    let start = original.len() - original.trim_start_matches([' ', '\t', '\n', '\r']).len();
    let end = original.trim_end_matches([' ', '\t', '\n', '\r']).len();
    let mut output = original.as_bytes()[..start].to_vec();
    output.extend(encode_preserving_source(&raw, current)?);
    output.extend_from_slice(&original.as_bytes()[end..]);
    Ok(output)
}

pub(super) fn encode_preserving_source(
    original: &serde_json::value::RawValue,
    current: &serde_json::Value,
) -> Result<Vec<u8>> {
    let previous: serde_json::Value = serde_json::from_str(original.get())?;
    if previous == *current {
        return Ok(original.get().as_bytes().to_vec());
    }
    match current {
        serde_json::Value::Object(object) if previous.is_object() => {
            let originals: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
                serde_json::from_str(original.get())?;
            let mut output = Vec::new();
            output.push(b'{');
            for (index, (key, value)) in object.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                serde_json::to_writer(&mut output, key)?;
                output.push(b':');
                match originals.get(key) {
                    Some(raw) => output.extend(encode_preserving_source(raw, value)?),
                    None => serde_json::to_writer(&mut output, value)?,
                }
            }
            output.push(b'}');
            Ok(output)
        }
        serde_json::Value::Array(items) if previous.is_array() => {
            let originals: Vec<Box<serde_json::value::RawValue>> =
                serde_json::from_str(original.get())?;
            let mut output = Vec::new();
            output.push(b'[');
            for (index, value) in items.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                match originals.get(index) {
                    Some(raw) => output.extend(encode_preserving_source(raw, value)?),
                    None => serde_json::to_writer(&mut output, value)?,
                }
            }
            output.push(b']');
            Ok(output)
        }
        _ => Ok(serde_json::to_vec(current)?),
    }
}
