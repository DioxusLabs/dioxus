use super::interface::PluginRuntimeState;
use async_trait::async_trait;
use ext_toml::Value;
use toml as ext_toml;
// Could these converts be simplified with bytemuck POD perhaps?

#[async_trait]
pub trait ConvertWithState<T> {
    async fn convert_with_state(self, state: &mut PluginRuntimeState) -> T;
}

pub trait Convert<T> {
    fn convert(self) -> T;
}

impl<T, U> Convert<Option<T>> for Option<U>
where
    U: Convert<T>,
{
    fn convert(self) -> Option<T> {
        self.map(Convert::convert)
    }
}

impl Convert<toml_edit::Offset> for toml::value::Offset {
    fn convert(self) -> toml_edit::Offset {
        match self {
            ext_toml::value::Offset::Z => toml_edit::Offset::Z,
            ext_toml::value::Offset::Custom { minutes, .. } => {
                toml_edit::Offset::Custom { minutes }
            }
        }
    }
}
impl Convert<toml_edit::Date> for toml::value::Date {
    fn convert(self) -> toml_edit::Date {
        let Self { year, month, day } = self;
        toml_edit::Date { year, month, day }
    }
}
impl Convert<toml_edit::Time> for toml::value::Time {
    fn convert(self) -> toml_edit::Time {
        let Self {
            hour,
            minute,
            second,
            nanosecond,
        } = self;
        toml_edit::Time {
            hour,
            minute,
            second,
            nanosecond,
        }
    }
}
impl Convert<toml_edit::Datetime> for toml::value::Datetime {
    fn convert(self) -> toml_edit::Datetime {
        let Self { date, time, offset } = self;
        toml_edit::Datetime {
            date: date.convert(),
            time: time.convert(),
            offset: offset.convert(),
        }
    }
}

impl Convert<toml_edit::Item> for toml::Value {
    fn convert(self) -> toml_edit::Item {
        let val: toml_edit::Value = self.convert();
        if let toml_edit::Value::InlineTable(table) = val {
            toml_edit::Item::Table(table.into_table())
        } else {
            toml_edit::value(val)
        }
    }
}

impl Convert<toml_edit::Value> for toml::Value {
    fn convert(self) -> toml_edit::Value {
        match self {
            Value::String(val) => val.into(),
            Value::Integer(val) => val.into(),
            Value::Float(val) => val.into(),
            Value::Boolean(val) => val.into(),
            Value::Datetime(val) => {
                toml_edit::Value::Datetime(toml_edit::Formatted::new(val.convert()))
            }
            Value::Array(arr) => {
                let arr = arr
                    .into_iter()
                    .fold(toml_edit::Array::new(), |mut acc, next| {
                        acc.push_formatted(next.convert());
                        acc
                    });
                arr.into()
            }
            Value::Table(map) => {
                let map =
                    map.into_iter()
                        .fold(toml_edit::InlineTable::new(), |mut acc, (key, item)| {
                            acc.insert(&key, item.convert());
                            acc
                        });
                map.into()
            }
        }
    }
}

// Redacted for now
// See issue: https://github.com/bytecodealliance/wit-bindgen/issues/817
// impl Convert<ext_toml::value::Datetime> for Datetime {
//     fn convert(self) -> ext_toml::value::Datetime {
//         let Datetime { date, time, offset } = self;
//         ext_toml::value::Datetime {
//             date: date.convert(),
//             time: time.convert(),
//             offset: offset.convert(),
//         }
//     }
// }
// impl Convert<Datetime> for ext_toml::value::Datetime {
//     fn convert(self) -> Datetime {
//         let ext_toml::value::Datetime { date, time, offset } = self;
//         Datetime {
//             date: date.convert(),
//             time: time.convert(),
//             offset: offset.convert(),
//         }
//     }
// }

// impl Convert<ext_toml::value::Time> for Time {
//     fn convert(self) -> ext_toml::value::Time {
//         let Time {
//             hour,
//             minute,
//             second,
//             nanosecond,
//         } = self;
//         ext_toml::value::Time {
//             hour,
//             minute,
//             second,
//             nanosecond,
//         }
//     }
// }
// impl Convert<Time> for ext_toml::value::Time {
//     fn convert(self) -> Time {
//         let ext_toml::value::Time {
//             hour,
//             minute,
//             second,
//             nanosecond,
//         } = self;
//         Time {
//             hour,
//             minute,
//             second,
//             nanosecond,
//         }
//     }
// }

// impl Convert<ext_toml::value::Date> for Date {
//     fn convert(self) -> ext_toml::value::Date {
//         let Date { year, month, day } = self;
//         ext_toml::value::Date { year, month, day }
//     }
// }

// impl Convert<ext_toml::value::Offset> for Offset {
//     fn convert(self) -> ext_toml::value::Offset {
//         match self {
//             Offset::Z => ext_toml::value::Offset::Z,
//             Offset::Custom((hours, minutes)) => ext_toml::value::Offset::Custom {
//                 minutes: (minutes as i16) + (hours * 80) as i16,
//             },
//         }
//     }
// }

// impl Convert<Date> for ext_toml::value::Date {
//     fn convert(self) -> Date {
//         let ext_toml::value::Date { year, month, day } = self;
//         Date { year, month, day }
//     }
// }

// // This is a bit ridiculous
// impl Convert<Offset> for ext_toml::value::Offset {
//     fn convert(self) -> Offset {
//         match self {
//             ext_toml::value::Offset::Z => Offset::Z,
//             ext_toml::value::Offset::Custom { minutes } => {
//                 Offset::Custom(((minutes / 60) as i8, (minutes % 60) as u8))
//             }
//         }
//     }
// }

// #[async_trait]
// impl ConvertWithState<Value> for TomlValue {
//     async fn convert_with_state(self, state: &mut PluginRuntimeState) -> Value {
//         match self {
//             TomlValue::String(string) => Value::String(string),
//             TomlValue::Integer(int) => Value::Integer(int),
//             TomlValue::Float(float) => Value::Float(float),
//             TomlValue::Boolean(b) => Value::Boolean(b),
//             TomlValue::Datetime(datetime) => Value::Datetime(datetime.convert()),
//             TomlValue::Array(array) => {
//                 let mut new_array = Vec::with_capacity(array.len());
//                 for item in array.into_iter() {
//                     new_array.push(state.get_toml(item).convert_with_state(state).await)
//                 }
//                 Value::Array(new_array)
//             }
//             TomlValue::Table(t) => {
//                 let mut table = Table::new();
//                 for (key, value) in t {
//                     let converted = state.get_toml(value).convert_with_state(state).await;
//                     table.insert(key, converted);
//                 }
//                 Value::Table(table)
//             }
//         }
//     }
// }

// #[async_trait]
// impl ConvertWithState<TomlValue> for Value {
//     async fn convert_with_state(self, state: &mut PluginRuntimeState) -> TomlValue {
//         match self {
//             Value::String(string) => TomlValue::String(string),
//             Value::Integer(int) => TomlValue::Integer(int),
//             Value::Float(float) => TomlValue::Float(float),
//             Value::Boolean(b) => TomlValue::Boolean(b),
//             Value::Datetime(d) => TomlValue::Datetime(d.convert()),
//             Value::Array(array) => {
//                 let mut new_arr = Vec::with_capacity(array.len());
//                 for item in array.into_iter() {
//                     new_arr.push(item.convert_with_state(state).await);
//                 }
//                 TomlValue::Array(new_arr)
//             }
//             Value::Table(list) => {
//                 let mut table = Vec::with_capacity(list.len());
//                 for (key, item) in list.into_iter() {
//                     table.push((key, item.convert_with_state(state).await));
//                 }
//                 TomlValue::Table(table)
//             }
//         }
//     }
// }

// #[async_trait]
// impl ConvertWithState<Resource<Toml>> for Value {
//     async fn convert_with_state(self, state: &mut PluginRuntimeState) -> Resource<Toml> {
//         let toml_value: TomlValue = self.convert_with_state(state).await;
//         toml_value.convert_with_state(state).await
//     }
// }

// #[async_trait]
// impl ConvertWithState<Resource<Toml>> for TomlValue {
//     async fn convert_with_state(self, state: &mut PluginRuntimeState) -> Resource<Toml> {
//         // This impl causes the set function add whole new toml's, check if it's
//         // already in state somehow?
//         state.new(self).await.unwrap()
//     }
// }
