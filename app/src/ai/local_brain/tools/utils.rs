use serde_json::Value;

pub fn string_array(value: &Value, key: &str) -> Option<Vec<String>> {
    value.get(key).and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    })
}

pub fn json_object_to_struct(value: &Value) -> Option<prost_types::Struct> {
    use std::collections::BTreeMap;
    match value {
        Value::Object(map) => Some(prost_types::Struct {
            fields: map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost_value(v)))
                .collect::<BTreeMap<_, _>>(),
        }),
        _ => Some(prost_types::Struct::default()),
    }
}

pub fn json_to_prost_value(value: &Value) -> prost_types::Value {
    use std::collections::BTreeMap;

    use prost_types::value::Kind;
    let kind = match value {
        Value::Null => Kind::NullValue(0),
        Value::Bool(b) => Kind::BoolValue(*b),
        Value::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => Kind::StringValue(s.clone()),
        Value::Array(a) => Kind::ListValue(prost_types::ListValue {
            values: a.iter().map(json_to_prost_value).collect(),
        }),
        Value::Object(o) => Kind::StructValue(prost_types::Struct {
            fields: o
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost_value(v)))
                .collect::<BTreeMap<_, _>>(),
        }),
    };
    prost_types::Value { kind: Some(kind) }
}

pub fn struct_to_json(s: &prost_types::Struct) -> Value {
    serde_json::Value::Object(
        s.fields
            .iter()
            .map(|(k, v)| (k.clone(), prost_value_to_json(v)))
            .collect(),
    )
}

pub fn prost_value_to_json(value: &prost_types::Value) -> Value {
    use prost_types::value::Kind;
    match &value.kind {
        Some(Kind::NullValue(_)) | None => Value::Null,
        Some(Kind::BoolValue(b)) => Value::Bool(*b),
        Some(Kind::NumberValue(n)) => serde_json::json!(n),
        Some(Kind::StringValue(s)) => Value::String(s.clone()),
        Some(Kind::ListValue(l)) => {
            Value::Array(l.values.iter().map(prost_value_to_json).collect())
        }
        Some(Kind::StructValue(s)) => struct_to_json(s),
    }
}

pub fn truncate_json_leaves(value: &mut Value, max_len: usize) {
    match value {
        Value::String(s) => {
            if s.chars().count() > max_len {
                let truncated: String = s.chars().take(max_len).collect();
                *s = format!(
                    "{truncated}\n… [truncated {} more chars]",
                    s.chars().count() - max_len
                );
            }
        }
        Value::Array(arr) => {
            for v in arr {
                truncate_json_leaves(v, max_len);
            }
        }
        Value::Object(obj) => {
            for v in obj.values_mut() {
                truncate_json_leaves(v, max_len);
            }
        }
        _ => {}
    }
}
