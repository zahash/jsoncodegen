use serde_json::{Map, Value};
use std::{fmt::Display, ops::Deref};

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

impl From<Value> for Schema {
    fn from(json: Value) -> Self {
        match json {
            Value::Array(arr) => Self::Array(array(arr)),
            Value::Object(obj) => Self::Object(object(obj)),
            _ => unreachable!("Valid top level Value will always be object or array"),
        }
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

            (FieldType::String, FieldType::Integer) | (FieldType::Integer, FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Integer])
            }
            (FieldType::String, FieldType::Float) | (FieldType::Float, FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Float])
            }
            (FieldType::String, FieldType::Boolean) | (FieldType::Boolean, FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Boolean])
            }
            (FieldType::Integer, FieldType::Float) | (FieldType::Float, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Float])
            }
            (FieldType::Integer, FieldType::Boolean) | (FieldType::Boolean, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Boolean])
            }
            (FieldType::Float, FieldType::Boolean) | (FieldType::Boolean, FieldType::Float) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Boolean])
            }

            (FieldType::String, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Object(fields)])
            }
            (FieldType::Integer, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Object(fields)])
            }
            (FieldType::Float, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::Float) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Object(fields)])
            }
            (FieldType::Boolean, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Object(fields)])
            }

            (FieldType::String, FieldType::Union(mut tys))
            | (FieldType::Union(mut tys), FieldType::String) => {
                if !tys.contains(&FieldType::String) {
                    tys.push(FieldType::String);
                }
                FieldType::Union(tys)
            }
            (FieldType::Integer, FieldType::Union(mut tys))
            | (FieldType::Union(mut tys), FieldType::Integer) => {
                if !tys.contains(&FieldType::Integer) {
                    tys.push(FieldType::Integer);
                }
                FieldType::Union(tys)
            }
            (FieldType::Float, FieldType::Union(mut tys))
            | (FieldType::Union(mut tys), FieldType::Float) => {
                if !tys.contains(&FieldType::Float) {
                    tys.push(FieldType::Float);
                }
                FieldType::Union(tys)
            }
            (FieldType::Boolean, FieldType::Union(mut tys))
            | (FieldType::Union(mut tys), FieldType::Boolean) => {
                if !tys.contains(&FieldType::Boolean) {
                    tys.push(FieldType::Boolean);
                }
                FieldType::Union(tys)
            }

            (FieldType::String, FieldType::Array(ty))
            | (FieldType::Array(ty), FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Array(ty)])
            }
            (FieldType::Integer, FieldType::Array(ty))
            | (FieldType::Array(ty), FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Array(ty)])
            }
            (FieldType::Float, FieldType::Array(ty)) | (FieldType::Array(ty), FieldType::Float) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Array(ty)])
            }
            (FieldType::Boolean, FieldType::Array(ty))
            | (FieldType::Array(ty), FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Array(ty)])
            }

            (FieldType::Optional(ty), FieldType::Unknown)
            | (FieldType::Unknown, FieldType::Optional(ty)) => FieldType::Optional(ty),
            (ft, FieldType::Unknown) | (FieldType::Unknown, ft) => {
                FieldType::Optional(Box::new(ft))
            }
            (FieldType::String, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::String) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::String, *ty)))
            }
            (FieldType::Integer, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Integer) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Integer, *ty)))
            }
            (FieldType::Float, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Float) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Float, *ty)))
            }
            (FieldType::Boolean, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Boolean) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Boolean, *ty)))
            }
            (FieldType::Object(fields), FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Object(fields)) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Object(fields), *ty)))
            }
            (FieldType::Union(union_types), FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Union(union_types)) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Union(union_types), *ty)))
            }
            (FieldType::Array(arr_ty), FieldType::Optional(op_ty))
            | (FieldType::Optional(op_ty), FieldType::Array(arr_ty)) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Array(arr_ty), *op_ty)))
            }

            (FieldType::Object(existing_fields), FieldType::Object(new_fields)) => {
                FieldType::Object(Self::merge_obj_fields(existing_fields, new_fields))
            }

            (FieldType::Object(obj_fields), FieldType::Union(mut union_types))
            | (FieldType::Union(mut union_types), FieldType::Object(obj_fields)) => {
                match union_types
                    .iter_mut()
                    .filter_map(|ty| match ty {
                        FieldType::Object(existing_obj_fields) => Some(existing_obj_fields),
                        _ => None,
                    })
                    .next()
                {
                    Some(existing_obj_fields) => match obj_fields == *existing_obj_fields {
                        true => FieldType::Union(union_types),
                        false => {
                            let merged_obj_fields =
                                Self::merge_obj_fields(existing_obj_fields.clone(), obj_fields);
                            *existing_obj_fields = merged_obj_fields;
                            FieldType::Union(union_types)
                        }
                    },
                    None => {
                        union_types.push(FieldType::Object(obj_fields));
                        FieldType::Union(union_types)
                    }
                }
            }
            (FieldType::Array(arr_type), FieldType::Union(mut union_types))
            | (FieldType::Union(mut union_types), FieldType::Array(arr_type)) => match union_types
                .iter_mut()
                .filter_map(|ty| match ty {
                    FieldType::Array(existing_arr_ty) => Some(existing_arr_ty),
                    _ => None,
                })
                .next()
            {
                Some(existing_arr_type) => match *existing_arr_type == arr_type {
                    true => FieldType::Union(union_types),
                    false => {
                        let merged_arr_type =
                            Self::merge(existing_arr_type.deref().deref().clone(), *arr_type);
                        *existing_arr_type = Box::new(merged_arr_type);
                        FieldType::Union(union_types)
                    }
                },
                None => {
                    union_types.push(FieldType::Array(arr_type));
                    FieldType::Union(union_types)
                }
            },

            (FieldType::Object(obj_fields), FieldType::Array(arr_ty))
            | (FieldType::Array(arr_ty), FieldType::Object(obj_fields)) => FieldType::Union(vec![
                FieldType::Object(obj_fields),
                FieldType::Array(arr_ty),
            ]),

            (FieldType::Union(existing_types), FieldType::Union(new_types)) => {
                let mut merged_types = existing_types;
                for new_type in new_types {
                    if !merged_types.contains(&new_type) {
                        merged_types.push(new_type);
                    }
                }
                FieldType::Union(merged_types)
            }

            (FieldType::Array(existing_ele_type), FieldType::Array(new_ele_type)) => {
                let merged_ele_type = Self::merge(*existing_ele_type, *new_ele_type);
                FieldType::Array(Box::new(merged_ele_type))
            }

            (FieldType::Optional(existing_ty), FieldType::Optional(new_ty)) => {
                FieldType::Optional(Box::new(Self::merge(*existing_ty, *new_ty)))
            }
        }
    }

    fn merge_obj_fields(mut existing_fields: Vec<Field>, mut new_fields: Vec<Field>) -> Vec<Field> {
        existing_fields = existing_fields
            .into_iter()
            .map(|mut existing_field| {
                match new_fields
                    .iter()
                    .find(|new_field| existing_field.name == new_field.name)
                {
                    Some(_) => existing_field,
                    None => {
                        existing_field.ty = FieldType::Optional(Box::new(existing_field.ty));
                        existing_field
                    }
                }
            })
            .collect();

        new_fields = new_fields
            .into_iter()
            .map(|mut new_field| {
                match existing_fields
                    .iter()
                    .find(|existing_field| existing_field.name == new_field.name)
                {
                    Some(_) => new_field,
                    None => {
                        new_field.ty = FieldType::Optional(Box::new(new_field.ty));
                        new_field
                    }
                }
            })
            .collect();

        let mut merged_fields = existing_fields;
        for new_field in new_fields {
            match merged_fields.iter_mut().find(|f| f.name == new_field.name) {
                Some(field) => field.ty = Self::merge(field.ty.clone(), new_field.ty),
                None => merged_fields.push(new_field),
            }
        }
        merged_fields
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
        Value::Number(n) => match n.is_u64() || n.is_i64() {
            true => FieldType::Integer,
            false => FieldType::Float,
        },
        Value::String(_) => FieldType::String,
        Value::Array(arr) => FieldType::Array(Box::new(array(arr))),
        Value::Object(obj) => FieldType::Object(object(obj)),
    }
}

impl Display for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Schema::Object(fields) => write!(f, "{{{}}}", FieldsDisp(fields)),
            Schema::Array(field_type) => write!(f, "[{}]", field_type),
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.ty)
    }
}

impl Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::String => write!(f, "str"),
            FieldType::Integer => write!(f, "int"),
            FieldType::Float => write!(f, "float"),
            FieldType::Boolean => write!(f, "bool"),
            FieldType::Unknown => write!(f, "null"),
            FieldType::Object(fields) => write!(f, "{{{}}}", FieldsDisp(fields)),
            FieldType::Union(field_types) => {
                for field_type in field_types {
                    write!(f, "|{}", field_type)?;
                }
                write!(f, "|")
            }
            FieldType::Array(field_type) => write!(f, "[{}]", field_type),
            FieldType::Optional(field_type) => write!(f, "{}?", field_type),
        }
    }
}

struct FieldsDisp<'a>(&'a [Field]);
impl Display for FieldsDisp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.iter();
        if let Some(field) = iter.next() {
            write!(f, "{}", field)?;
            for field in iter {
                write!(f, ",{}", field)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn check(json: &str, schema: &str) {
        let json = serde_json::from_str::<Value>(json).expect("invalid json string");
        assert_eq!(format!("{}", Schema::from(json)), schema);
    }

    #[test]
    fn empty() {
        check("{}", "{}");
        check("[]", "[null]");
    }

    #[test]
    fn array() {
        check(
            r#"
                [
                    "mixed", null, true, 123, 123.23,
                    ["nested", "arr"], ["arr2"], [123], [true, 27, [22.34]],
                    {"k1": "v1", "k3": true}, {"k1": 23, "k3": false}, {"k2": "v2", "k3": true}
                ]
                "#,
            "[|str|bool|int|float|[|str|int|bool|[float]|]|{k1:|str|int|?,k3:bool,k2:str?}|?]",
        );
    }

    #[test]
    fn object() {
        check(
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
                        "mixed", null, true, 123, 123.23,
                        ["nested", "arr"], ["arr2"], [123], [true, 27, [22.34]],
                        {"k1": "v1", "k3": true}, {"k1": 23, "k3": false}, {"k2": "v2", "k3": true}
                    ]
                }
                "#,
            "{a:str,b:int,c:float,d:bool,e:null,f:{n:str},g:[int],\
            h:[|str|bool|int|float|[|str|int|bool|[float]|]|{k1:|str|int|?,k3:bool,k2:str?}|?]}",
        );
    }
}
