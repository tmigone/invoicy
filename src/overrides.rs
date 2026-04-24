use toml::Value;

pub fn apply(config: &mut Value, override_str: &str) -> Result<(), String> {
    let (key, value) = override_str.split_once('=').ok_or_else(|| {
        format!(
            "Invalid override format: '{}'. Expected KEY=VALUE",
            override_str
        )
    })?;

    let existing_type = get_value_type(config, key);
    let parsed_value = parse_value_with_hint(value, existing_type);
    set_nested_value(config, key, parsed_value)
}

#[derive(Clone, Copy)]
enum ValueType {
    String,
    Integer,
    Float,
    Boolean,
    Unknown,
}

fn get_value_type(config: &Value, key: &str) -> ValueType {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = config;

    for part in &parts {
        if let Some((name, idx_str)) = parse_array_access(part) {
            if let Some(arr) = current.get(name).and_then(|v| v.as_array()) {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Some(elem) = arr.get(idx) {
                        current = elem;
                        continue;
                    }
                }
            }
            return ValueType::Unknown;
        } else if let Some(next) = current.get(*part) {
            current = next;
        } else {
            return ValueType::Unknown;
        }
    }

    match current {
        Value::String(_) => ValueType::String,
        Value::Integer(_) => ValueType::Integer,
        Value::Float(_) => ValueType::Float,
        Value::Boolean(_) => ValueType::Boolean,
        _ => ValueType::Unknown,
    }
}

fn parse_value_with_hint(s: &str, hint: ValueType) -> Value {
    match hint {
        ValueType::String => Value::String(s.to_string()),
        ValueType::Integer => s
            .parse::<i64>()
            .map(Value::Integer)
            .unwrap_or_else(|_| Value::String(s.to_string())),
        ValueType::Float => s
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or_else(|_| Value::String(s.to_string())),
        ValueType::Boolean => match s {
            "true" | "1" => Value::Boolean(true),
            "false" | "0" => Value::Boolean(false),
            _ => Value::String(s.to_string()),
        },
        ValueType::Unknown => parse_value_auto(s),
    }
}

fn parse_value_auto(s: &str) -> Value {
    if s == "true" {
        return Value::Boolean(true);
    }
    if s == "false" {
        return Value::Boolean(false);
    }
    // Preserve leading zeros as strings (e.g., "00000123")
    if !s.starts_with('0') || s == "0" {
        if let Ok(i) = s.parse::<i64>() {
            return Value::Integer(i);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
    }
    Value::String(s.to_string())
}

fn set_nested_value(config: &mut Value, key: &str, value: Value) -> Result<(), String> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = config;

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if let Some((name, idx_str)) = parse_array_access(part) {
            let idx: usize = idx_str
                .parse()
                .map_err(|_| format!("Invalid array index: {}", idx_str))?;

            let arr = current
                .get_mut(name)
                .ok_or_else(|| format!("Key '{}' not found", name))?
                .as_array_mut()
                .ok_or_else(|| format!("'{}' is not an array", name))?;

            while arr.len() <= idx {
                arr.push(Value::Table(toml::map::Map::new()));
            }

            if is_last {
                arr[idx] = value;
                return Ok(());
            } else {
                current = &mut arr[idx];
            }
        } else {
            if is_last {
                current
                    .as_table_mut()
                    .ok_or_else(|| format!("Cannot set '{}': parent is not a table", part))?
                    .insert(part.to_string(), value);
                return Ok(());
            } else {
                current = current
                    .get_mut(*part)
                    .ok_or_else(|| format!("Key '{}' not found", part))?;
            }
        }
    }

    Ok(())
}

fn parse_array_access(s: &str) -> Option<(&str, &str)> {
    let open = s.find('[')?;
    let close = s.find(']')?;
    if close > open + 1 {
        Some((&s[..open], &s[open + 1..close]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // parse_value_auto tests
    // ==========================================================================

    #[test]
    fn parse_value_auto_boolean() {
        assert_eq!(parse_value_auto("true"), Value::Boolean(true));
        assert_eq!(parse_value_auto("false"), Value::Boolean(false));
    }

    #[test]
    fn parse_value_auto_integer() {
        assert_eq!(parse_value_auto("42"), Value::Integer(42));
        assert_eq!(parse_value_auto("-10"), Value::Integer(-10));
        assert_eq!(parse_value_auto("0"), Value::Integer(0));
    }

    #[test]
    fn parse_value_auto_float() {
        assert_eq!(parse_value_auto("3.14"), Value::Float(3.14));
        assert_eq!(parse_value_auto("-2.5"), Value::Float(-2.5));
    }

    #[test]
    fn parse_value_auto_string() {
        assert_eq!(parse_value_auto("hello"), Value::String("hello".into()));
        assert_eq!(parse_value_auto(""), Value::String("".into()));
    }

    #[test]
    fn parse_value_auto_leading_zeros_stay_string() {
        assert_eq!(parse_value_auto("00123"), Value::String("00123".into()));
        assert_eq!(parse_value_auto("007"), Value::String("007".into()));
    }

    // ==========================================================================
    // parse_array_access tests
    // ==========================================================================

    #[test]
    fn parse_array_access_valid() {
        assert_eq!(parse_array_access("items[0]"), Some(("items", "0")));
        assert_eq!(parse_array_access("arr[123]"), Some(("arr", "123")));
    }

    #[test]
    fn parse_array_access_invalid() {
        assert_eq!(parse_array_access("items"), None);
        assert_eq!(parse_array_access("items[]"), None);
        assert_eq!(parse_array_access("items["), None);
    }

    // ==========================================================================
    // apply tests (integration)
    // ==========================================================================

    fn sample_config() -> Value {
        toml::from_str(
            r#"
            name = "test"
            count = 42
            price = 9.99
            enabled = true

            [nested]
            value = "inner"

            [[items]]
            desc = "first"
            qty = 1

            [[items]]
            desc = "second"
            qty = 2
            "#,
        )
        .unwrap()
    }

    #[test]
    fn apply_simple_string() {
        let mut config = sample_config();
        apply(&mut config, "name=updated").unwrap();
        assert_eq!(config.get("name").unwrap().as_str().unwrap(), "updated");
    }

    #[test]
    fn apply_string_field_with_number_value() {
        let mut config = sample_config();
        // name is a string field, so 123 should stay as string
        apply(&mut config, "name=123").unwrap();
        assert_eq!(config.get("name").unwrap().as_str().unwrap(), "123");
    }

    #[test]
    fn apply_integer_field() {
        let mut config = sample_config();
        apply(&mut config, "count=100").unwrap();
        assert_eq!(config.get("count").unwrap().as_integer().unwrap(), 100);
    }

    #[test]
    fn apply_float_field() {
        let mut config = sample_config();
        apply(&mut config, "price=19.99").unwrap();
        assert_eq!(config.get("price").unwrap().as_float().unwrap(), 19.99);
    }

    #[test]
    fn apply_boolean_field() {
        let mut config = sample_config();
        apply(&mut config, "enabled=false").unwrap();
        assert_eq!(config.get("enabled").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn apply_nested_value() {
        let mut config = sample_config();
        apply(&mut config, "nested.value=changed").unwrap();
        assert_eq!(
            config
                .get("nested")
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap(),
            "changed"
        );
    }

    #[test]
    fn apply_array_element() {
        let mut config = sample_config();
        apply(&mut config, "items[0].desc=modified").unwrap();
        assert_eq!(
            config.get("items").unwrap().as_array().unwrap()[0]
                .get("desc")
                .unwrap()
                .as_str()
                .unwrap(),
            "modified"
        );
    }

    #[test]
    fn apply_array_element_integer() {
        let mut config = sample_config();
        apply(&mut config, "items[1].qty=99").unwrap();
        assert_eq!(
            config.get("items").unwrap().as_array().unwrap()[1]
                .get("qty")
                .unwrap()
                .as_integer()
                .unwrap(),
            99
        );
    }

    #[test]
    fn apply_invalid_format() {
        let mut config = sample_config();
        let result = apply(&mut config, "invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid override format"));
    }

    #[test]
    fn apply_missing_key() {
        let mut config = sample_config();
        let result = apply(&mut config, "nonexistent.field=value");
        assert!(result.is_err());
    }
}
