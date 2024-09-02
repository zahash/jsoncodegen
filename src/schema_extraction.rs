use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Schema {
    Object(Vec<Field>),
    Array(FieldType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Unknown,
    Object(Vec<Field>),
    Union(Vec<FieldType>),
    Array(Box<FieldType>),
    Optional(Box<FieldType>),
}

pub fn extract(json: Value) -> Schema {
    match json {
        Value::Array(arr) => Schema::Array(array(arr)),
        Value::Object(obj) => Schema::Object(object(obj)),
        _ => unreachable!("Valid top level Value will always be object or array"),
    }
}

fn object(obj: Map<String, Value>) -> Vec<Field> {
    let mut fields = vec![];

    for (key, value) in obj {
        fields.push(Field {
            name: key,
            ty: field_type(value),
        });
    }

    fields
}

struct FieldTypeAggregator {
    ty: Option<FieldType>,
}

impl FieldTypeAggregator {
    fn new() -> Self {
        Self { ty: None }
    }

    fn add(&mut self, field_type: FieldType) {
        match self.ty.take() {
            None => self.ty = Some(field_type),
            Some(ty) => self.ty = Some(Self::merge(ty, field_type)),
        };
    }

    fn finalize(self) -> FieldType {
        self.ty.unwrap_or(FieldType::Unknown)
    }

    fn merge(existing: FieldType, new: FieldType) -> FieldType {
        match (existing, new) {
            (FieldType::String, FieldType::String) => FieldType::String,
            (FieldType::Integer, FieldType::Integer) => FieldType::Integer,
            (FieldType::Float, FieldType::Float) => FieldType::Float,
            (FieldType::Boolean, FieldType::Boolean) => FieldType::Boolean,
            (FieldType::Unknown, FieldType::Unknown) => FieldType::Unknown,

            (FieldType::String, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::String, FieldType::Integer])
            }
            (FieldType::String, FieldType::Float) => {
                FieldType::Union(vec![FieldType::String, FieldType::Float])
            }
            (FieldType::String, FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::String, FieldType::Boolean])
            }
            (FieldType::Integer, FieldType::String) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::String])
            }
            (FieldType::Integer, FieldType::Float) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Float])
            }
            (FieldType::Integer, FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Boolean])
            }
            (FieldType::Float, FieldType::String) => {
                FieldType::Union(vec![FieldType::Float, FieldType::String])
            }
            (FieldType::Float, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Integer])
            }
            (FieldType::Float, FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Boolean])
            }
            (FieldType::Boolean, FieldType::String) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::String])
            }
            (FieldType::Boolean, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Integer])
            }
            (FieldType::Boolean, FieldType::Float) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Float])
            }

            (FieldType::String, FieldType::Unknown) => {
                FieldType::Optional(Box::new(FieldType::String))
            }
            (FieldType::Integer, FieldType::Unknown) => {
                FieldType::Optional(Box::new(FieldType::Integer))
            }
            (FieldType::Float, FieldType::Unknown) => {
                FieldType::Optional(Box::new(FieldType::Float))
            }
            (FieldType::Boolean, FieldType::Unknown) => {
                FieldType::Optional(Box::new(FieldType::Boolean))
            }

            (FieldType::String, FieldType::Object(fields)) => {
                FieldType::Union(vec![FieldType::String, FieldType::Object(fields)])
            }
            (FieldType::Integer, FieldType::Object(fields)) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Object(fields)])
            }
            (FieldType::Float, FieldType::Object(fields)) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Object(fields)])
            }
            (FieldType::Boolean, FieldType::Object(fields)) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Object(fields)])
            }

            (FieldType::String, FieldType::Union(mut tys)) => {
                if !tys.contains(&FieldType::String) {
                    tys.push(FieldType::String);
                }
                FieldType::Union(tys)
            }
            (FieldType::Integer, FieldType::Union(mut tys)) => {
                if !tys.contains(&FieldType::Integer) {
                    tys.push(FieldType::Integer);
                }
                FieldType::Union(tys)
            }
            (FieldType::Float, FieldType::Union(mut tys)) => {
                if !tys.contains(&FieldType::Float) {
                    tys.push(FieldType::Float);
                }
                FieldType::Union(tys)
            }

            (FieldType::String, FieldType::Array(ty)) => {
                FieldType::Union(vec![FieldType::String, FieldType::Array(ty)])
            }
            (FieldType::Integer, FieldType::Array(ty)) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Array(ty)])
            }
            (FieldType::Float, FieldType::Array(ty)) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Array(ty)])
            }

            (FieldType::String, FieldType::Optional(ty)) => {
                FieldType::Optional(Box::new(FieldType::Union(vec![FieldType::String, *ty])))
            }
            (FieldType::Integer, FieldType::Optional(ty)) => {
                FieldType::Optional(Box::new(FieldType::Union(vec![FieldType::Integer, *ty])))
            }
            (FieldType::Float, FieldType::Optional(ty)) => {
                FieldType::Optional(Box::new(FieldType::Union(vec![FieldType::Float, *ty])))
            }

            (FieldType::Boolean, FieldType::Union(_)) => todo!(),
            (FieldType::Boolean, FieldType::Array(_)) => todo!(),
            (FieldType::Boolean, FieldType::Optional(_)) => todo!(),
            (FieldType::Unknown, FieldType::String) => todo!(),
            (FieldType::Unknown, FieldType::Integer) => todo!(),
            (FieldType::Unknown, FieldType::Float) => todo!(),
            (FieldType::Unknown, FieldType::Boolean) => todo!(),
            (FieldType::Unknown, FieldType::Object(_)) => todo!(),
            (FieldType::Unknown, FieldType::Union(_)) => todo!(),
            (FieldType::Unknown, FieldType::Array(_)) => todo!(),
            (FieldType::Unknown, FieldType::Optional(_)) => todo!(),
            (FieldType::Object(_), FieldType::String) => todo!(),
            (FieldType::Object(_), FieldType::Integer) => todo!(),
            (FieldType::Object(_), FieldType::Float) => todo!(),
            (FieldType::Object(_), FieldType::Boolean) => todo!(),
            (FieldType::Object(_), FieldType::Unknown) => todo!(),

            (FieldType::Object(existing_fields), FieldType::Object(new_fields)) => {
                let mut merged_fields = existing_fields;
                for new_field in new_fields {
                    match merged_fields.iter_mut().find(|f| f.name == new_field.name) {
                        Some(field) => field.ty = Self::merge(field.ty.clone(), new_field.ty),
                        None => merged_fields.push(new_field),
                    }
                }
                FieldType::Object(merged_fields)
            }

            (FieldType::Object(_), FieldType::Union(_)) => todo!(),
            (FieldType::Object(_), FieldType::Array(_)) => todo!(),
            (FieldType::Object(_), FieldType::Optional(_)) => todo!(),
            (FieldType::Union(_), FieldType::String) => todo!(),
            (FieldType::Union(_), FieldType::Integer) => todo!(),
            (FieldType::Union(_), FieldType::Float) => todo!(),
            (FieldType::Union(_), FieldType::Boolean) => todo!(),
            (FieldType::Union(_), FieldType::Unknown) => todo!(),
            (FieldType::Union(_), FieldType::Object(_)) => todo!(),

            (FieldType::Union(existing_types), FieldType::Union(new_types)) => {
                let mut merged_types = existing_types;
                for new_type in new_types {
                    if !merged_types.contains(&new_type) {
                        merged_types.push(new_type);
                    }
                }
                FieldType::Union(merged_types)
            }

            (FieldType::Union(_), FieldType::Array(_)) => todo!(),
            (FieldType::Union(_), FieldType::Optional(_)) => todo!(),
            (FieldType::Array(_), FieldType::String) => todo!(),
            (FieldType::Array(_), FieldType::Integer) => todo!(),
            (FieldType::Array(_), FieldType::Float) => todo!(),
            (FieldType::Array(_), FieldType::Boolean) => todo!(),
            (FieldType::Array(_), FieldType::Unknown) => todo!(),
            (FieldType::Array(_), FieldType::Object(_)) => todo!(),
            (FieldType::Array(_), FieldType::Union(_)) => todo!(),

            (FieldType::Array(existing_ele_type), FieldType::Array(new_ele_type)) => {
                let merged_ele_type = Self::merge(*existing_ele_type, *new_ele_type);
                FieldType::Array(Box::new(merged_ele_type))
            }

            (FieldType::Array(_), FieldType::Optional(_)) => todo!(),
            (FieldType::Optional(_), FieldType::String) => todo!(),
            (FieldType::Optional(_), FieldType::Integer) => todo!(),
            (FieldType::Optional(_), FieldType::Float) => todo!(),
            (FieldType::Optional(_), FieldType::Boolean) => todo!(),
            (FieldType::Optional(_), FieldType::Unknown) => todo!(),
            (FieldType::Optional(_), FieldType::Object(_)) => todo!(),
            (FieldType::Optional(_), FieldType::Union(_)) => todo!(),
            (FieldType::Optional(_), FieldType::Array(_)) => todo!(),
            (FieldType::Optional(_), FieldType::Optional(_)) => todo!(),
            // (FieldType::Union(existing_types), new_type) => {
            //     let mut merged_types = existing_types;
            //     if !merged_types.contains(&new_type) {
            //         merged_types.push(new_type);
            //     }
            //     FieldType::Union(merged_types)
            // }
            // (existing_type, FieldType::Union(new_types)) => {
            //     let mut merged_types = new_types;
            //     if !merged_types.contains(&existing_type) {
            //         merged_types.push(existing_type);
            //     }
            //     FieldType::Union(merged_types)
            // }
            // (existing_type, new_type) => match existing_type == new_type {
            //     true => existing_type,
            //     false => FieldType::Union(vec![existing_type, new_type]),
            // },
        }
    }
}

fn array(arr: Vec<Value>) -> FieldType {
    let mut agg = FieldTypeAggregator::new();

    for value in arr {
        let field_type = field_type(value);
        agg.add(field_type);
    }

    agg.finalize()
}

fn field_type(value: Value) -> FieldType {
    match value {
        Value::Null => FieldType::Unknown,
        Value::Bool(_) => FieldType::Boolean,
        Value::Number(n) => match n.is_f64() {
            true => FieldType::Float,
            false => FieldType::Integer,
        },
        Value::String(_) => FieldType::String,
        Value::Array(arr) => FieldType::Array(Box::new(array(arr))),
        Value::Object(obj) => FieldType::Object(object(obj)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn empty() {
        assert_eq!(extract(json("{}")), Schema::Object(vec![]));
        assert_eq!(extract(json("[]")), Schema::Array(FieldType::Unknown));
    }

    #[test]
    fn array() {
        let json = json(
            r#"
                {
                    "h": [
                        "mixed", true, 
                        ["nested", "arr"], ["arr2"], [123], [true, 27, [22.34]], 
                        {"k1": "v1", "k3": true}, {"k1": 23, "k3": false}, {"k2": "v2", "k3": true}
                    ]
                }
                "#,
        );

        let schema = extract(json);

        println!("{:#?}", schema);
    }

    #[test]
    fn object() {
        let json = json(
            r#"
                {
                    "a": "amogus",
                    "b": 123,
                    "c": 45.67,
                    "d": true,
                    "e": null,
                    "f": {"n": "nested"},
                    "g": [1, 2],
                    "h": [
                        "mixed", true, 
                        ["nested", "arr"], ["arr2"], [123], [true, 27, [22.34]], 
                        {"k1": "v1", "k3": true}, {"k1": 23, "k3": false}, {"k2": "v2", "k3": true}
                    ]
                }
                "#,
        );

        let schema = extract(json);

        println!("{:#?}", schema);

        assert_eq!(
            schema,
            Schema::Object(vec![
                Field {
                    name: "a".into(),
                    ty: FieldType::String
                },
                Field {
                    name: "b".into(),
                    ty: FieldType::Integer
                },
                Field {
                    name: "c".into(),
                    ty: FieldType::Float
                },
                Field {
                    name: "d".into(),
                    ty: FieldType::Boolean
                },
                Field {
                    name: "e".into(),
                    ty: FieldType::Unknown
                },
                Field {
                    name: "f".into(),
                    ty: FieldType::Object(vec![Field {
                        name: "n".into(),
                        ty: FieldType::String
                    }])
                },
                Field {
                    name: "g".into(),
                    ty: FieldType::Array(Box::new(FieldType::Integer))
                },
                Field {
                    name: "h".into(),
                    ty: FieldType::Array(Box::new(FieldType::Union(vec![
                        FieldType::String,
                        FieldType::Boolean,
                        FieldType::Array(Box::new(FieldType::Union(vec![
                            FieldType::String,
                            FieldType::Boolean,
                            FieldType::Integer,
                            FieldType::Array(Box::new(FieldType::Float))
                        ]))),
                        FieldType::Object(vec![
                            Field {
                                name: "k1".into(),
                                ty: FieldType::Optional(Box::new(FieldType::Union(vec![
                                    FieldType::String,
                                    FieldType::Integer
                                ])))
                            },
                            Field {
                                name: "k2".into(),
                                ty: FieldType::Optional(Box::new(FieldType::String))
                            },
                            Field {
                                name: "k3".into(),
                                ty: FieldType::Boolean
                            }
                        ])
                    ])))
                },
            ])
        );
    }
}
