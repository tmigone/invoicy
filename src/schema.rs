use schemars::{JsonSchema, schema_for};
use serde_json::Value;

struct FieldInfo {
    path: String,
    typ: String,
    optional: bool,
}

fn extract_fields(schema: &Value, definitions: &Value, prefix: &str, fields: &mut Vec<FieldInfo>) {
    let Some(obj) = schema.as_object() else {
        return;
    };

    // Handle $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            extract_fields(def_schema, definitions, prefix, fields);
        }
        return;
    }

    // Handle object type with properties
    if let Some(properties) = obj.get("properties").and_then(|v| v.as_object()) {
        let required: std::collections::HashSet<&str> = obj
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        for (name, prop_schema) in properties {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", prefix, name)
            };
            let is_optional = !required.contains(name.as_str());

            // Check if this is a nested object or array
            if is_nested_object(prop_schema, definitions) {
                extract_fields(prop_schema, definitions, &path, fields);
            } else if is_array(prop_schema, definitions) {
                extract_array_fields(prop_schema, definitions, &path, fields);
            } else if is_optional_array(prop_schema, definitions) {
                extract_optional_array_fields(prop_schema, definitions, &path, fields);
            } else {
                let typ = get_type_name(prop_schema, definitions);
                fields.push(FieldInfo {
                    path,
                    typ,
                    optional: is_optional,
                });
            }
        }
    }
}

fn extract_array_fields(
    schema: &Value,
    definitions: &Value,
    prefix: &str,
    fields: &mut Vec<FieldInfo>,
) {
    let Some(obj) = schema.as_object() else {
        return;
    };

    // Follow $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            extract_array_fields(def_schema, definitions, prefix, fields);
        }
        return;
    }

    // Get items schema
    if let Some(items) = obj.get("items") {
        let item_prefix = format!("{}[]", prefix);
        extract_fields(items, definitions, &item_prefix, fields);
    }
}

fn is_nested_object(schema: &Value, definitions: &Value) -> bool {
    let Some(obj) = schema.as_object() else {
        return false;
    };

    // Follow $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            return is_nested_object(def_schema, definitions);
        }
        return false;
    }

    // Check if it's an object with properties
    obj.get("properties").is_some()
}

fn is_array(schema: &Value, definitions: &Value) -> bool {
    let Some(obj) = schema.as_object() else {
        return false;
    };

    // Follow $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            return is_array(def_schema, definitions);
        }
        return false;
    }

    obj.get("type").and_then(|v| v.as_str()) == Some("array")
}

fn is_optional_array(schema: &Value, definitions: &Value) -> bool {
    let Some(obj) = schema.as_object() else {
        return false;
    };

    // Check for type array like ["array", "null"]
    if let Some(types) = obj.get("type").and_then(|v| v.as_array()) {
        let has_array = types.iter().any(|t| t.as_str() == Some("array"));
        let has_null = types.iter().any(|t| t.as_str() == Some("null"));
        if has_array && has_null {
            return true;
        }
    }

    // Check for anyOf with array type
    if let Some(any_of) = obj.get("anyOf").and_then(|v| v.as_array()) {
        for variant in any_of {
            if is_array(variant, definitions) {
                return true;
            }
        }
    }

    false
}

fn extract_optional_array_fields(
    schema: &Value,
    definitions: &Value,
    prefix: &str,
    fields: &mut Vec<FieldInfo>,
) {
    let Some(obj) = schema.as_object() else {
        return;
    };

    // Get items schema from the array type
    if let Some(items) = obj.get("items") {
        let item_prefix = format!("{}[]", prefix);
        extract_fields(items, definitions, &item_prefix, fields);
    }
}

fn get_type_name(schema: &Value, definitions: &Value) -> String {
    let Some(obj) = schema.as_object() else {
        return "unknown".to_string();
    };

    // Follow $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            return get_type_name(def_schema, definitions);
        }
    }

    // Handle anyOf (for Option types)
    if let Some(any_of) = obj.get("anyOf").and_then(|v| v.as_array()) {
        for variant in any_of {
            let typ = variant.get("type").and_then(|v| v.as_str());
            if typ != Some("null") {
                if let Some(t) = typ {
                    return normalize_type(t);
                }
                // If no direct type, check for $ref
                if let Some(ref_path) = variant.get("$ref").and_then(|v| v.as_str()) {
                    let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
                    if let Some(def_schema) = definitions.get(def_name) {
                        return get_type_name(def_schema, definitions);
                    }
                }
            }
        }
    }

    // Direct type (string)
    if let Some(typ) = obj.get("type").and_then(|v| v.as_str()) {
        return normalize_type(typ);
    }

    // Type array (for Option types like ["string", "null"])
    if let Some(types) = obj.get("type").and_then(|v| v.as_array()) {
        for typ in types {
            if let Some(t) = typ.as_str()
                && t != "null"
            {
                return normalize_type(t);
            }
        }
    }

    "unknown".to_string()
}

fn normalize_type(typ: &str) -> String {
    match typ {
        "string" => "string".to_string(),
        "number" | "integer" => "number".to_string(),
        "boolean" => "boolean".to_string(),
        "array" => "array".to_string(),
        "object" => "object".to_string(),
        "null" => "null".to_string(),
        _ => typ.to_string(),
    }
}

pub fn print_schema<T: JsonSchema>(format_name: &str) {
    let root_schema = schema_for!(T);
    let json = serde_json::to_value(&root_schema).expect("Failed to serialize schema");
    let definitions = json
        .get("$defs")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let mut fields = Vec::new();
    extract_fields(&json, &definitions, "", &mut fields);

    println!("Format: {}\n", format_name);

    let max_path_len = fields.iter().map(|f| f.path.len()).max().unwrap_or(0);

    for field in fields {
        let optional = if field.optional { " (optional)" } else { "" };
        println!(
            "  {:<width$}  {}{}",
            field.path,
            field.typ,
            optional,
            width = max_path_len
        );
    }
}
