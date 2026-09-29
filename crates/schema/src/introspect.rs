//! Flatten a format's JSON schema into dotted field paths and simple types.
//!
//! Paths use `a.b` for nested tables and `items[].x` for array elements, the
//! same shape `--set` overrides take (with `[]` standing for any index).

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde_json::Value;

use crate::{AfipCInvoice, GenericInvoice};

/// One leaf field of a format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Dotted path, e.g. `comprobante.numero` or `items[].subtotal`.
    pub path: String,
    /// `string`, `number`, `boolean`, … (JSON-schema type names).
    pub typ: String,
    pub optional: bool,
    /// Accepted values, for enum fields (e.g. `productos`, `servicios`, …).
    pub values: Vec<String>,
    /// Value used when the field is omitted, if it has one.
    pub default: Option<String>,
    /// Where an automatically filled field comes from (`emisor.toml`, `AFIP`,
    /// `computed`); `None` for fields written in the invoice TOML.
    pub source: Option<String>,
}

fn extract_fields(
    schema: &Value,
    definitions: &Value,
    prefix: &str,
    source: Option<&str>,
    fields: &mut Vec<Field>,
) {
    let Some(obj) = schema.as_object() else {
        return;
    };

    // Handle $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            extract_fields(def_schema, definitions, prefix, source, fields);
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
            // schemars puts a field's default and extensions next to its
            // `$ref`, so read them before looking through the reference. A
            // table's source applies to every field in it.
            let source = prop_schema
                .get("x-source")
                .and_then(Value::as_str)
                .or(source);
            let default = prop_schema.get("default").map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            });
            // `Option<Struct>` is `anyOf: [<struct>, null]`; look through it.
            let prop_schema = non_null_variant(prop_schema).unwrap_or(prop_schema);
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", prefix, name)
            };
            let is_optional = !required.contains(name.as_str());

            // Check if this is a nested object or array
            if is_nested_object(prop_schema, definitions) {
                extract_fields(prop_schema, definitions, &path, source, fields);
            } else if is_array(prop_schema, definitions) {
                extract_array_fields(prop_schema, definitions, &path, source, fields);
            } else if is_optional_array(prop_schema, definitions) {
                extract_optional_array_fields(prop_schema, definitions, &path, source, fields);
            } else {
                let typ = get_type_name(prop_schema, definitions);
                fields.push(Field {
                    path,
                    typ,
                    optional: is_optional,
                    values: enum_values(prop_schema, definitions),
                    default,
                    source: source.map(str::to_string),
                });
            }
        }
    }
}

/// The accepted values of an enum field: a string `enum`, or a `oneOf` of
/// `const`s (what schemars emits once variants carry doc comments).
fn enum_values(schema: &Value, definitions: &Value) -> Vec<String> {
    let schema = match schema.get("$ref").and_then(Value::as_str) {
        Some(ref_path) => {
            let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
            match definitions.get(def_name) {
                Some(def) => def,
                None => return Vec::new(),
            }
        }
        None => schema,
    };
    let strings = |values: &Vec<Value>| -> Vec<String> {
        values
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return strings(values);
    }
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        let consts: Vec<Value> = variants
            .iter()
            .filter_map(|v| v.get("const").cloned())
            .collect();
        if consts.len() == variants.len() {
            return strings(&consts);
        }
    }
    Vec::new()
}

/// The single non-null alternative of an `anyOf`, if that is its shape.
fn non_null_variant(schema: &Value) -> Option<&Value> {
    let any_of = schema.get("anyOf")?.as_array()?;
    let mut variants = any_of
        .iter()
        .filter(|v| v.get("type").and_then(Value::as_str) != Some("null"));
    let variant = variants.next()?;
    variants.next().is_none().then_some(variant)
}

fn extract_array_fields(
    schema: &Value,
    definitions: &Value,
    prefix: &str,
    source: Option<&str>,
    fields: &mut Vec<Field>,
) {
    let Some(obj) = schema.as_object() else {
        return;
    };

    // Follow $ref
    if let Some(ref_path) = obj.get("$ref").and_then(|v| v.as_str()) {
        let def_name = ref_path.strip_prefix("#/$defs/").unwrap_or(ref_path);
        if let Some(def_schema) = definitions.get(def_name) {
            extract_array_fields(def_schema, definitions, prefix, source, fields);
        }
        return;
    }

    // Get items schema
    if let Some(items) = obj.get("items") {
        let item_prefix = format!("{}[]", prefix);
        extract_fields(items, definitions, &item_prefix, source, fields);
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
    source: Option<&str>,
    fields: &mut Vec<Field>,
) {
    let Some(obj) = schema.as_object() else {
        return;
    };

    // Get items schema from the array type
    if let Some(items) = obj.get("items") {
        let item_prefix = format!("{}[]", prefix);
        extract_fields(items, definitions, &item_prefix, source, fields);
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
        "number" => "number".to_string(),
        "integer" => "integer".to_string(),
        "boolean" => "boolean".to_string(),
        "array" => "array".to_string(),
        "object" => "object".to_string(),
        "null" => "null".to_string(),
        _ => typ.to_string(),
    }
}

/// All leaf fields of `T`, in schema order: the ones written in the TOML and
/// the output-only ones filled in automatically (see [`Field::source`]).
pub fn fields<T: JsonSchema>() -> Vec<Field> {
    // Output-only fields are `skip_deserializing`, so only the serialize
    // schema lists them; whether a written field is optional (and its default)
    // is a property of reading, so that comes from the deserialize schema.
    let mut fields = extract::<T>(SchemaSettings::default().for_serialize());
    let input = extract::<T>(SchemaSettings::default().for_deserialize());
    for field in &mut fields {
        match input.iter().find(|f| f.path == field.path) {
            Some(written) => {
                field.optional = written.optional;
                field.default = written.default.clone();
            }
            None => {
                field.optional = false;
                field.default = None;
            }
        }
    }
    fields
}

fn extract<T: JsonSchema>(settings: SchemaSettings) -> Vec<Field> {
    let root_schema = settings.into_generator().into_root_schema_for::<T>();
    let json = serde_json::to_value(&root_schema).expect("Failed to serialize schema");
    let definitions = json
        .get("$defs")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let mut fields = Vec::new();
    extract_fields(&json, &definitions, "", None, &mut fields);
    fields
}

/// All leaf fields of the format called `format`, or `None` if unknown.
pub fn format_fields(format: &str) -> Option<Vec<Field>> {
    match format {
        "generic" => Some(fields::<GenericInvoice>()),
        "afip_c" => Some(fields::<AfipCInvoice>()),
        _ => None,
    }
}

/// Look up the expected type for a field path given a format name
pub fn field_type(format: &str, path: &str) -> Option<String> {
    let fields = format_fields(format)?;

    // Try exact match first, then array item match
    // (e.g., "items[0].description" -> "items[].description")
    let normalized = normalize_array_path(path);
    fields
        .iter()
        .find(|f| f.path == path)
        .or_else(|| fields.iter().find(|f| f.path == normalized))
        .map(|f| f.typ.clone())
}

/// Convert "items[0].field" to "items[].field" for schema lookup
fn normalize_array_path(path: &str) -> String {
    let mut result = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '[' {
            result.push('[');
            // Skip digits until ]
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    break;
                }
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}
