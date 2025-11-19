mod type_table;

use crate::schema::{Field, FieldType, Schema};
use serde_json::Value;
use type_table::TypeTable;

#[derive(Debug, Clone, PartialEq)]
pub struct FlatSchema {
    pub root: Root,
    pub objects: Vec<Object>,
    pub unions: Vec<Union>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Root {
    Object(usize),
    Array(FlatFieldType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: usize,
    pub fields: Vec<FlatField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Union {
    pub id: usize,
    pub field_types: Vec<FlatFieldType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlatField {
    pub name: String,
    pub ty: FlatFieldType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlatFieldType {
    String,
    Integer,
    Float,
    Boolean,
    Unknown,
    Object(usize),
    Union(usize),
    Array(Box<FlatFieldType>),
    Optional(Box<FlatFieldType>),
}

impl FlatSchema {
    pub fn object<'a>(&'a self, id: usize) -> Option<&'a Object> {
        self.objects.iter().find(|obj| obj.id == id)
    }

    pub fn union<'a>(&'a self, id: usize) -> Option<&'a Union> {
        self.unions.iter().find(|un| un.id == id)
    }
}

impl From<Value> for FlatSchema {
    fn from(json: Value) -> Self {
        Self::from(Schema::from(json))
    }
}

impl From<Schema> for FlatSchema {
    fn from(schema: Schema) -> Self {
        let mut type_table = TypeTable::new();

        let root = match schema {
            Schema::Object(fields) => Root::Object(flatten_object(fields, &mut type_table)),
            Schema::Array(field_type) => {
                Root::Array(flatten_field_type(field_type, &mut type_table))
            }
        };

        let (objects, unions) = type_table.types();

        Self {
            root,
            objects,
            unions,
        }
    }
}

fn flatten_object(fields: Vec<Field>, type_table: &mut TypeTable) -> usize {
    let flat_fields = fields
        .into_iter()
        .map(|field| FlatField {
            name: field.name,
            ty: flatten_field_type(field.ty, type_table),
        })
        .collect();

    type_table.register_object(flat_fields)
}

fn flatten_union(field_types: Vec<FieldType>, type_table: &mut TypeTable) -> usize {
    let flat_field_types = field_types
        .into_iter()
        .map(|field_type| flatten_field_type(field_type, type_table))
        .collect();

    type_table.register_union(flat_field_types)
}

fn flatten_field_type(field_type: FieldType, type_table: &mut TypeTable) -> FlatFieldType {
    match field_type {
        FieldType::String => FlatFieldType::String,
        FieldType::Integer => FlatFieldType::Integer,
        FieldType::Float => FlatFieldType::Float,
        FieldType::Boolean => FlatFieldType::Boolean,
        FieldType::Unknown => FlatFieldType::Unknown,
        FieldType::Object(fields) => FlatFieldType::Object(flatten_object(fields, type_table)),
        FieldType::Union(field_types) => {
            FlatFieldType::Union(flatten_union(field_types, type_table))
        }
        FieldType::Array(field_type) => {
            FlatFieldType::Array(Box::new(flatten_field_type(*field_type, type_table)))
        }
        FieldType::Optional(field_type) => {
            FlatFieldType::Optional(Box::new(flatten_field_type(*field_type, type_table)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::Value;

    #[inline(always)]
    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn empty() {
        assert_eq!(
            FlatSchema::from(json("{}")),
            FlatSchema {
                root: Root::Object(0),
                objects: vec![Object {
                    id: 0,
                    fields: vec![]
                }],
                unions: vec![]
            }
        );

        assert_eq!(
            FlatSchema::from(json("[]")),
            FlatSchema {
                root: Root::Array(FlatFieldType::Unknown),
                objects: vec![],
                unions: vec![]
            }
        );
    }

    #[test]
    fn test() {
        println!(
            "{:#?}",
            FlatSchema::from(json(
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
                "#
            ))
        )
    }
}
