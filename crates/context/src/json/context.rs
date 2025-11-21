use spanner::json::JsonValue;
use spanner::json as types;
use spanner::Spanned;

use serde::{Serialize, Serializer};
use serde_json::Value as SerdeJsonValue;
use std::collections::HashMap;

/// A verifier of contextual integrity for JSON.
///
/// See the [module level documentation](crate::context) for more information.
#[derive(Debug)]
pub struct JsonContext {
    value: JsonValue,
}

impl JsonContext {
    /// Creates a new builder.
    pub fn builder(
        value: JsonValue,
    ) -> JsonContextBuilder {
        JsonContextBuilder::new(value)
    }
}

/// Builder for [`JsonContext`].
pub struct JsonContextBuilder {
    value: JsonValue,
}

impl JsonContextBuilder {
    /// Creates a new builder.
    pub fn new(
        value: JsonValue,
    ) -> Self {
        Self { value }
    }

    /// Builds the context.
    pub fn build(self) -> Result<JsonContext, Box<dyn std::error::Error>> {
        Ok(JsonContext { value: self.value })
    }
}

/// A visitor for JSON values that converts spansy::json::JsonValue to serde_json::Value.
pub trait JsonSerializationVisitor {
    /// Visit a JSON value and convert it to serde_json::Value.
    fn visit_value(&mut self, value: &JsonValue) -> SerdeJsonValue {
        match value {
            JsonValue::Null(_) => SerdeJsonValue::Null,
            JsonValue::Redacted(_) => SerdeJsonValue::String("__REDACTED__".to_string()),
            JsonValue::Bool(b) => SerdeJsonValue::Bool(b.span().as_str().parse().unwrap_or(false)),
            JsonValue::Number(n) => {
                let num_str = n.span().as_str();
                if num_str.contains('.') {
                    num_str.parse::<f64>().map(|f| SerdeJsonValue::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))).unwrap_or(SerdeJsonValue::String(num_str.to_string()))
                } else {
                    num_str.parse::<i64>().map(|n| SerdeJsonValue::Number(serde_json::Number::from(n))).unwrap_or(SerdeJsonValue::String(num_str.to_string()))
                }
            },
            JsonValue::String(s) => SerdeJsonValue::String(s.span().as_str().to_string()),
            JsonValue::Array(arr) => self.visit_array(arr),
            JsonValue::Object(obj) => self.visit_object(obj),
        }
    }

    /// Visit a JSON object and convert it to serde_json::Value.
    fn visit_object(&mut self, obj: &types::Object) -> SerdeJsonValue {
        let mut map = HashMap::new();
        for elem in obj.elems.iter() {
            let key = elem.key.span().as_str().to_string();
            let value = self.visit_value(&elem.value);
            map.insert(key, value);
        }
        SerdeJsonValue::Object(serde_json::Map::from_iter(map))
    }

    /// Visit a JSON array and convert it to serde_json::Value.
    fn visit_array(&mut self, arr: &types::Array) -> SerdeJsonValue {
        let values: Vec<SerdeJsonValue> = arr.elems.iter().map(|elem| self.visit_value(elem)).collect();
        SerdeJsonValue::Array(values)
    }
}

/// The default JSON serialization visitor.
#[derive(Debug, Default)]
pub struct DefaultJsonSerializationVisitor {}

impl JsonSerializationVisitor for DefaultJsonSerializationVisitor {}

impl Serialize for JsonContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut visitor = DefaultJsonSerializationVisitor::default();
        let serde_value = visitor.visit_value(&self.value);
        serde_value.serialize(serializer)
    }
}

