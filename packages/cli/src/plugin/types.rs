use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub available: bool,
    pub loader: Vec<String>,
    pub config_info: HashMap<String, HashMap<String, Value>>,
}

impl PluginConfig {
    pub fn from_toml_value(val: toml::Value) -> Self {
        if let toml::Value::Table(tab) = val {
            let available = tab
                .get::<_>("available")
                .unwrap_or(&toml::Value::Boolean(true));
            let available = available.as_bool().unwrap_or(true);

            let mut loader = vec![];
            if let Some(origin) = tab.get("loader") {
                if origin.is_array() {
                    for i in origin.as_array().unwrap() {
                        loader.push(i.as_str().unwrap_or_default().to_string());
                    }
                }
            }

            let mut config_info = HashMap::new();

            for (name, value) in tab {
                if name == "available" || name == "loader" {
                    continue;
                }
                if let toml::Value::Table(value) = value {
                    let mut map = HashMap::new();
                    for (item, info) in value {
                        map.insert(item, Value::from_toml(info));
                    }
                    config_info.insert(name, map);
                }
            }

            Self {
                available,
                loader,
                config_info,
            }
        } else {
            Self {
                available: false,
                loader: vec![],
                config_info: HashMap::new(),
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Table(HashMap<String, Value>),
}

impl Value {
    pub fn from_toml(origin: toml::Value) -> Self {
        match origin {
            cargo_toml::Value::String(s) => Value::String(s),
            cargo_toml::Value::Integer(i) => Value::Integer(i),
            cargo_toml::Value::Float(f) => Value::Float(f),
            cargo_toml::Value::Boolean(b) => Value::Boolean(b),
            cargo_toml::Value::Datetime(d) => Value::String(d.to_string()),
            cargo_toml::Value::Array(a) => {
                let mut v = vec![];
                for i in a {
                    v.push(Value::from_toml(i));
                }
                Value::Array(v)
            }
            cargo_toml::Value::Table(t) => {
                let mut h = HashMap::new();
                for (n, v) in t {
                    h.insert(n, Value::from_toml(v));
                }
                Value::Table(h)
            }
        }
    }
}
