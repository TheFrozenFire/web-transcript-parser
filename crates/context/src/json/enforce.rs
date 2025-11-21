use spanner::json::JsonValue;
use spanner::json as types;
use spanner::Spanned;

/// A visitor for JSON values that checks for structural integrity.
pub trait JsonContextEnforcer {
    /// Visit a JSON value.
    fn visit_value(&mut self, structure: &JsonValue, value: &JsonValue) {
        // Ensure value has same variant as structure
        match (structure, value) {
            (JsonValue::Null(_), JsonValue::Null(_)) => (),
            (JsonValue::Redacted(_), _) => (),

            (JsonValue::Bool(structure), JsonValue::Bool(value)) => assert_eq!(structure.span().as_str(), value.span().as_str()),
            (JsonValue::Number(structure), JsonValue::Number(value)) => assert_eq!(structure.span().as_str(), value.span().as_str()),
            (JsonValue::String(structure), JsonValue::String(value)) => assert_eq!(structure.span().as_str(), value.span().as_str()),


            (JsonValue::Array(structure), JsonValue::Array(value)) => self.visit_array(structure, value),
            (JsonValue::Object(structure), JsonValue::Object(value)) => self.visit_object(structure, value),

            _ => panic!("Type mismatch: expected {:?}, got {:?}", structure, value)
        }
    }

    /// Visit a JSON object.
    fn visit_object(&mut self, structure: &types::Object, value: &types::Object) {
        for elem in structure.elems.iter() {
            let matching = value.elems.iter().find(|e| e.key.span().as_str() == elem.key.span().as_str());
            assert!(matching.is_some(), "Missing key: {}", elem.key.span().as_str());

            self.visit_value(&elem.value, &matching.unwrap().value);
        }
    }

    /// Visit a JSON array.
    /// The default strategy enforces that if the array is non-empty, then the
    /// value must be an array with the same number of elements, and that each
    /// non-redacted element is structurally equivalent to the corresponding
    /// element in the structure.
    fn visit_array(&mut self, structure: &types::Array, value: &types::Array) {
        for (i, elem) in structure.elems.iter().enumerate() {
            self.visit_value(elem, &value.elems[i]);
        }
    }
}

/// The default JSON context visitor.
#[derive(Debug, Default)]
pub struct DefaultJsonContextEnforcer {}

impl JsonContextEnforcer for DefaultJsonContextEnforcer { }