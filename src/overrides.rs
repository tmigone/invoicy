use toml::Value;

pub fn apply(config: &mut Value, override_str: &str) -> Result<(), String> {
    let (key, value) = override_str
        .split_once('=')
        .ok_or_else(|| format!("Invalid override format: '{}'. Expected KEY=VALUE", override_str))?;

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
