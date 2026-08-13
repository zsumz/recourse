//! Explicit marker boundary for caller-visible structured evidence.

use std::{
    borrow::Cow,
    fmt::{self, Debug, Formatter},
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{IgnoredAny, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
};

/// Marks a schema-governed type as reviewed for public serialization.
///
/// This trait is deliberately not blanket-implemented. An implementation is
/// an explicit declaration that the type contains caller-visible protocol data
/// rather than private source-error material.
///
/// Retained string `format` keywords are runtime assertions. Catalog
/// construction rejects formats outside
/// [`SUPPORTED_SCHEMA_FORMATS`](crate::catalog::SUPPORTED_SCHEMA_FORMATS).
/// Schemars numeric formats are retained only on number/integer schemas; JSON
/// type and range constraints enforce their values.
/// Numeric evidence must serialize through Rust's primitive integer or finite
/// float methods. [`serde_json::Number`] and numeric [`serde_json::Value`]
/// variants are intentionally unsupported because they use serializer-private
/// raw-number tokens outside this governed boundary.
///
/// Private reports cannot cross this boundary:
///
/// ```compile_fail
/// use recourse::{diagnostic::PublicEvidence, fault::PrivateReport};
///
/// fn assert_public<T: PublicEvidence>() {}
/// assert_public::<PrivateReport>();
/// ```
pub trait PublicEvidence: Serialize + JsonSchema + Debug + Send + Sync + 'static {}

/// Empty public evidence, represented on the wire as `{}`.
///
/// ```
/// use recourse::diagnostic::NoEvidence;
///
/// # fn example() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::to_string(&NoEvidence)?, "{}");
/// # Ok(())
/// # }
/// # assert!(example().is_ok());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoEvidence;

/// Wire and schema description kept identical to the type's own documentation.
const NO_EVIDENCE_DESCRIPTION: &str = "Empty public evidence, represented on the wire as `{}`.";

impl PublicEvidence for NoEvidence {}

impl Serialize for NoEvidence {
    /// Emits `{}` rather than the `null` a unit-struct serializer would write.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_struct("NoEvidence", 0)?.end()
    }
}

impl<'de> Deserialize<'de> for NoEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("NoEvidence", &[], NoEvidenceVisitor)
    }
}

/// Accepts the shapes a fieldless struct has always accepted, ignoring
/// unknown members so evidence may gain properties compatibly.
struct NoEvidenceVisitor;

impl<'de> Visitor<'de> for NoEvidenceVisitor {
    type Value = NoEvidence;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("an empty evidence object")
    }

    fn visit_seq<A>(self, _elements: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        Ok(NoEvidence)
    }

    fn visit_map<A>(self, mut members: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while members.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(NoEvidence)
    }
}

impl JsonSchema for NoEvidence {
    fn schema_name() -> Cow<'static, str> {
        "NoEvidence".into()
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed(concat!(module_path!(), "::NoEvidence"))
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": NO_EVIDENCE_DESCRIPTION,
            "type": "object"
        })
    }
}
