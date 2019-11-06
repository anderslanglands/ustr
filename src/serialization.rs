use super::*;

use serde::{
    de::{Deserialize, Deserializer, SeqAccess, Visitor},
    ser::{Serialize, SerializeSeq, Serializer},
};

impl Serialize for Bins {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<&'static str> = string_cache_iter().collect();
        let mut seq = serializer.serialize_seq(Some(strings.len()))?;
        for s in strings {
            seq.serialize_element(s)?;
        }
        seq.end()
    }
}

pub struct BinsVisitor {}

impl BinsVisitor {
    pub fn new() -> Self {
        BinsVisitor {}
    }
}

impl<'de> Visitor<'de> for BinsVisitor {
    type Value = StringCachePlaceholder;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of strings")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(s) = seq.next_element::<String>()? {
            ustr(&s);
        }

        Ok(StringCachePlaceholder {})
    }
}

pub struct StringCachePlaceholder {}

impl<'de> Deserialize<'de> for StringCachePlaceholder {
    fn deserialize<D>(deserializer: D) -> Result<StringCachePlaceholder, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BinsVisitor::new())
    }
}
