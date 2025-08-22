use super::*;
use serde::{
    de::{Deserialize, Deserializer, Error, SeqAccess, Visitor},
    ser::{Serialize, SerializeSeq, Serializer},
};

impl<N: StringCacheNs> Serialize for Bins<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<&'static str> = string_cache_iter().collect();
        let mut seq = serializer.serialize_seq(Some(strings.len()))?;
        for s in strings {
            match seq.serialize_element(s) {
                Ok(_) => (),
                Err(e) => {
                    panic!("Error serializing \"{}\": {}", s, e);
                }
            }
        }
        seq.end()
    }
}

pub struct BinsVisitor {}

impl BinsVisitor {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        BinsVisitor {}
    }
}

impl<'de> Visitor<'de> for BinsVisitor {
    type Value = DeserializedCache;

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

        Ok(DeserializedCache {})
    }
}

pub struct DeserializedCache {}

impl<'de> Deserialize<'de> for DeserializedCache {
    fn deserialize<D>(deserializer: D) -> Result<DeserializedCache, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BinsVisitor::new())
    }
}

pub struct UstrVisitor<N: StringCacheNs> {
    __phantom: PhantomData<N>,
}

impl<N: StringCacheNs> UstrVisitor<N> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        UstrVisitor {
            __phantom: Default::default(),
        }
    }
}

impl<'de, N: StringCacheNs> Visitor<'de> for UstrVisitor<N> {
    type Value = InternedString<N>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a &str")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(InternedString::from(s))
    }
}

impl<'de, N: StringCacheNs> Deserialize<'de> for InternedString<N> {
    fn deserialize<D>(deserializer: D) -> Result<InternedString<N>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(UstrVisitor::new())
    }
}

impl<N: StringCacheNs> Serialize for InternedString<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
