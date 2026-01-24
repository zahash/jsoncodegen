//! # JSON Schema Inference
//!
//! This module provides functionality to infer type schemas from JSON data through structural analysis.
//! It takes arbitrary JSON input and produces a canonical schema representation that captures the
//! complete type structure, including unions, optionals, and nested objects.
//!
//! ## Core Data Structures
//!
//! ### [`Schema`]
//!
//! The top-level schema representation wrapping a single [`FieldType`].
//!
//! ### [`Field`]
//!
//! A named field within an object, consisting of:
//! - `name`: The field name as a string
//! - `ty`: The inferred type of the field as a [`FieldType`]
//!
//! ### [`FieldType`]
//!
//! The module distinguishes between several categories of types:
//!
//! #### Primitive Types
//! - `Boolean`, `Integer`, `Float`, `String` - Basic JSON value types
//!
//! #### Special Types
//! - `Null` - Represents an explicit JSON `null` value
//! - `Unknown` - Represents lack of type information (e.g., element type of an empty array `[]`)
//!
//! #### Structural Types
//! - Object(Vec<[Field]>) - Named field collection
//! - Array(Box<[FieldType]>) - Homogeneous collection
//!
//! #### Composite Types
//! - Optional(Box<[FieldType]>) - Type that can be the inner type or null
//! - Union(Vec<[FieldType]>) - Type that can be one of several alternatives
//!
//! ## Type Merging Semantics
//!
//! When multiple JSON values are analyzed (e.g., elements in an array), their types are merged:
//!
//! Eg:
//! - **T + T → T**: Same types merge to themselves
//! - **Unknown + T → T**: Unknown represents no information, so it adopts any concrete type
//! - **Null + T → Optional\<T\>**: Null indicates absence, making the type optional
//! - **T1 + T2 → Union\<T1, T2\>**: Different concrete types create a union
//!
//! For detailed merging rules, see [`FieldTypeAggregator::merge`].
//!
//! ## Examples
//!
//! ```rust
//! use serde_json::json;
//! use jsoncodegen::schema::Schema;
//!
//! // Simple object
//! let json = json!({"name": "Alice", "age": 30});
//! let schema = Schema::from(json);
//! // Result: {age:int, name:str}
//!
//! // Array with mixed types creates a union
//! let json = json!([1, "hello", 2.5]);
//! let schema = Schema::from(json);
//! // Result: [|int|float|str|]
//!
//! // Null values create optional types
//! let json = json!([1, null, 3]);
//! let schema = Schema::from(json);
//! // Result: [int?]
//!
//! // Empty arrays have unknown element type
//! let json = json!([]);
//! let schema = Schema::from(json);
//! // Result: [unknown]
//! ```

use serde_json::{Map, Value};
use std::fmt::Display;

/// Top-level schema wrapping the inferred type.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub ty: FieldType,
}

/// A named field within an object type.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
}

/// Inferred type of a JSON value.
///
/// for merging semantics, see [`FieldTypeAggregator::merge`].
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Unknown, // Represents a truly unknown/uninferred type (e.g., element type of an empty array `[]`)
    Null,    // Represents an explicit JSON `null` value
    Boolean,
    Integer,
    Float,
    String,
    Array(Box<FieldType>),    // JSON array
    Object(Vec<Field>),       // JSON object
    Optional(Box<FieldType>), // nullable type (can be inner type or null)
    Union(Vec<FieldType>),    // Union of heterogeneous types.
}

impl From<Value> for Schema {
    fn from(json: Value) -> Self {
        let mut field_type = field_type(json);
        // recursively sort field_type to make sure it has a deterministic order
        sort_field_type(&mut field_type);

        Schema { ty: field_type }
    }
}

/// Sorts fields alphabetically by field names and recursively sorts nested types.
fn sort_fields(fields: &mut [Field]) {
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    for field in fields {
        sort_field_type(&mut field.ty);
    }
}

/// Sorts types by complexity (Unknown < primitives < structures < modifiers) and recursively.
fn sort_field_types(field_types: &mut [FieldType]) {
    field_types.sort_by_key(|t| match t {
        FieldType::Unknown => 0,
        FieldType::Null => 1,
        FieldType::Boolean => 2, // Simplest primitive type
        FieldType::Integer => 3, // Numeric types ordered by specificity
        FieldType::Float => 4,   // More general numeric type
        FieldType::String => 5,
        FieldType::Array(_) => 6, // Collection types before complex structures
        FieldType::Object(_) => 7, // Complex structured type
        FieldType::Optional(_) => 8, // Wrapper types that modify other types
        FieldType::Union(_) => 9, // Most complex - union of multiple types
    });
    for field_type in field_types {
        sort_field_type(field_type);
    }
}

/// Recursively sorts a field type and nested components.
fn sort_field_type(field_type: &mut FieldType) {
    match field_type {
        FieldType::Object(fields) => sort_fields(fields),
        FieldType::Union(field_types) => sort_field_types(field_types),
        FieldType::Array(inner_field_type) | FieldType::Optional(inner_field_type) => {
            sort_field_type(inner_field_type)
        }
        _ => {}
    }
}

/// Converts JSON object to vector of typed fields.
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

/// Merges multiple types into a unified type by accumulating them.
///
/// Starts with `Unknown`, then merges each type sequentially.
struct FieldTypeAggregator {
    ty: FieldType,
}

impl FieldTypeAggregator {
    fn new() -> Self {
        Self {
            ty: FieldType::Unknown,
        }
    }

    /// Merges a new type into the accumulator (zero-copy via mem::replace).
    fn add(&mut self, field_type: FieldType) {
        self.ty = Self::merge(
            std::mem::replace(&mut self.ty, FieldType::Unknown),
            field_type,
        );
    }

    fn finalize(self) -> FieldType {
        self.ty
    }

    /// Core type merging algorithm. See the function body for detailed rules.
    ///
    /// - **T + T → T**: Same types merge to themselves
    /// - **Unknown + T → T**: Unknown represents no information, so it adopts any concrete type
    /// - **Null + T → Optional\<T\>**: Null indicates absence, making the type optional
    /// - **Null + Optional\<T\> → Optional\<T\>**: Null merged with an Optional remains Optional
    /// - **T1 + T2 → Union\<T1, T2\>**: Different concrete types create a union
    /// - Arrays/Objects merge recursively, Unions expand.
    fn merge(existing: FieldType, new: FieldType) -> FieldType {
        match (existing, new) {
            (FieldType::Unknown, FieldType::Unknown) => FieldType::Unknown,
            (FieldType::Null, FieldType::Null) => FieldType::Null,

            (FieldType::Boolean, FieldType::Boolean) => FieldType::Boolean,
            (FieldType::Integer, FieldType::Integer) => FieldType::Integer,
            (FieldType::Float, FieldType::Float) => FieldType::Float,
            (FieldType::String, FieldType::String) => FieldType::String,

            // Unknown represents lack of information, so it adopts the concrete type
            (ty, FieldType::Unknown) | (FieldType::Unknown, ty) => ty,

            // Null indicates absence of value, so it makes the type optional
            (ty, FieldType::Null) | (FieldType::Null, ty) => match ty {
                FieldType::Optional(_) => ty,
                _ => FieldType::Optional(Box::new(ty)),
            },

            // Primitive, Primitive
            (FieldType::Boolean, FieldType::Integer) | (FieldType::Integer, FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Integer])
            }
            (FieldType::Boolean, FieldType::Float) | (FieldType::Float, FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Float])
            }
            (FieldType::Boolean, FieldType::String) | (FieldType::String, FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::String])
            }
            (FieldType::Integer, FieldType::Float) | (FieldType::Float, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Float])
            }
            (FieldType::Integer, FieldType::String) | (FieldType::String, FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::String])
            }
            (FieldType::Float, FieldType::String) | (FieldType::String, FieldType::Float) => {
                FieldType::Union(vec![FieldType::Float, FieldType::String])
            }

            // Primitive, Array
            (FieldType::Boolean, FieldType::Array(ty))
            | (FieldType::Array(ty), FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Array(ty)])
            }
            (FieldType::Integer, FieldType::Array(ty))
            | (FieldType::Array(ty), FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Array(ty)])
            }
            (FieldType::Float, FieldType::Array(ty)) | (FieldType::Array(ty), FieldType::Float) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Array(ty)])
            }
            (FieldType::String, FieldType::Array(ty))
            | (FieldType::Array(ty), FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Array(ty)])
            }

            // Primitive, Object
            (FieldType::Boolean, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::Boolean) => {
                FieldType::Union(vec![FieldType::Boolean, FieldType::Object(fields)])
            }
            (FieldType::Integer, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::Integer) => {
                FieldType::Union(vec![FieldType::Integer, FieldType::Object(fields)])
            }
            (FieldType::Float, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::Float) => {
                FieldType::Union(vec![FieldType::Float, FieldType::Object(fields)])
            }
            (FieldType::String, FieldType::Object(fields))
            | (FieldType::Object(fields), FieldType::String) => {
                FieldType::Union(vec![FieldType::String, FieldType::Object(fields)])
            }

            // Primitive, Optional
            (FieldType::Boolean, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Boolean) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Boolean, *ty)))
            }
            (FieldType::Integer, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Integer) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Integer, *ty)))
            }
            (FieldType::Float, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Float) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Float, *ty)))
            }
            (FieldType::String, FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::String) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::String, *ty)))
            }

            // Primitive, Union
            (FieldType::Boolean, FieldType::Union(mut tys))
            | (FieldType::Union(mut tys), FieldType::Boolean) => {
                if !tys.contains(&FieldType::Boolean) {
                    tys.push(FieldType::Boolean);
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
            (FieldType::String, FieldType::Union(mut tys))
            | (FieldType::Union(mut tys), FieldType::String) => {
                if !tys.contains(&FieldType::String) {
                    tys.push(FieldType::String);
                }
                FieldType::Union(tys)
            }

            // Array, Array
            (FieldType::Array(existing_ele_type), FieldType::Array(new_ele_type)) => {
                let merged_ele_type = Self::merge(*existing_ele_type, *new_ele_type);
                FieldType::Array(Box::new(merged_ele_type))
            }

            // Object, Object
            (FieldType::Object(existing_fields), FieldType::Object(new_fields)) => {
                FieldType::Object(Self::merge_obj_fields(existing_fields, new_fields))
            }

            // Optional, Optional
            (FieldType::Optional(existing_ty), FieldType::Optional(new_ty)) => {
                FieldType::Optional(Box::new(Self::merge(*existing_ty, *new_ty)))
            }

            // Union, Union
            (FieldType::Union(existing_types), FieldType::Union(new_types)) => {
                let mut merged_types = existing_types;
                for new_type in new_types {
                    if !merged_types.contains(&new_type) {
                        merged_types.push(new_type);
                    }
                }
                FieldType::Union(merged_types)
            }

            // Array, Object
            (FieldType::Array(arr_ty), FieldType::Object(obj_fields))
            | (FieldType::Object(obj_fields), FieldType::Array(arr_ty)) => FieldType::Union(vec![
                FieldType::Object(obj_fields),
                FieldType::Array(arr_ty),
            ]),

            // Non-Primitive, Optional
            (FieldType::Array(arr_ty), FieldType::Optional(op_ty))
            | (FieldType::Optional(op_ty), FieldType::Array(arr_ty)) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Array(arr_ty), *op_ty)))
            }
            (FieldType::Object(fields), FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Object(fields)) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Object(fields), *ty)))
            }
            (FieldType::Union(union_types), FieldType::Optional(ty))
            | (FieldType::Optional(ty), FieldType::Union(union_types)) => {
                FieldType::Optional(Box::new(Self::merge(FieldType::Union(union_types), *ty)))
            }

            // Non-Primitive, Union
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
                        let yanked =
                            std::mem::replace(existing_arr_type, Box::new(FieldType::Unknown));
                        let merged_arr_type = Self::merge(*yanked, *arr_type);
                        *existing_arr_type = Box::new(merged_arr_type);
                        FieldType::Union(union_types)
                    }
                },
                None => {
                    union_types.push(FieldType::Array(arr_type));
                    FieldType::Union(union_types)
                }
            },
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
                            let yanked = std::mem::replace(existing_obj_fields, vec![]);
                            let merged_obj_fields = Self::merge_obj_fields(yanked, obj_fields);
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
        }
    }

    /// Merges object field lists: shared fields merge recursively, disjoint fields become Optional.
    fn merge_obj_fields(mut existing_fields: Vec<Field>, mut new_fields: Vec<Field>) -> Vec<Field> {
        existing_fields = existing_fields
            .into_iter()
            .map(|mut existing_field| {
                match new_fields
                    .iter()
                    .any(|new_field| existing_field.name == new_field.name)
                {
                    true => existing_field,
                    false => match existing_field.ty {
                        FieldType::Null | FieldType::Unknown | FieldType::Optional(_) => {
                            existing_field
                        }
                        _ => {
                            existing_field.ty = FieldType::Optional(Box::new(existing_field.ty));
                            existing_field
                        }
                    },
                }
            })
            .collect();

        new_fields = new_fields
            .into_iter()
            .map(|mut new_field| {
                match existing_fields
                    .iter()
                    .any(|existing_field| existing_field.name == new_field.name)
                {
                    true => new_field,
                    false => match new_field.ty {
                        FieldType::Null | FieldType::Unknown | FieldType::Optional(_) => new_field,
                        _ => {
                            new_field.ty = FieldType::Optional(Box::new(new_field.ty));
                            new_field
                        }
                    },
                }
            })
            .collect();

        let mut merged_fields = existing_fields;
        for new_field in new_fields {
            match merged_fields.iter_mut().find(|f| f.name == new_field.name) {
                Some(field) => {
                    let yanked = std::mem::replace(&mut field.ty, FieldType::Unknown);
                    field.ty = Self::merge(yanked, new_field.ty);
                }
                None => merged_fields.push(new_field),
            }
        }
        merged_fields
    }
}

/// Infers array element type by merging all elements.
fn array(arr: Vec<Value>) -> FieldType {
    let mut agg = FieldTypeAggregator::new();

    for value in arr {
        let field_type = field_type(value);
        agg.add(field_type);
    }

    agg.finalize()
}

/// Converts JSON Value to FieldType. Numbers are Integer if i64/u64, else Float.
fn field_type(value: Value) -> FieldType {
    match value {
        Value::Null => FieldType::Null,
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
        write!(f, "{}", self.ty)
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
            FieldType::Null => write!(f, "null"),
            FieldType::Unknown => write!(f, "unknown"),
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

/// Helper for comma-separated field display.
struct FieldsDisp<'a>(&'a [Field]);

impl Display for FieldsDisp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let [first, rest @ ..] = self.0 {
            write!(f, "{}", first)?;
            for field in rest {
                write!(f, ",{}", field)?;
            }
        }
        Ok(())
    }
}
