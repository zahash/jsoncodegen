use super::Iota;
use crate::schema::{Field, FieldType, Schema};

#[derive(Debug)]
pub struct Code {
    root: usize,
    types: Vec<Type>,
}

#[derive(Debug)]
pub struct Type {
    id: usize,
    ty: TypeType,
    trace: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum TypeType {
    String,
    Integer,
    Float,
    Boolean,
    Unknown,
    Object(Vec<ObjField>),
    Union(Vec<usize>),
    Array(usize),
    Optional(usize),
}

#[derive(Debug, PartialEq)]
pub struct ObjField {
    pub key: String,
    pub type_id: usize,
}

fn foo() {
    let _ = vec![Type {
        id: 1,
        trace: vec!["from".into()],
        ty: TypeType::Object(vec![
            ObjField {
                key: "x".into(),
                type_id: 0,
            },
            ObjField {
                key: "y".into(),
                type_id: 0,
            },
        ]),
    }];
}

struct Ctx {
    types: Vec<Type>,
    trace: Vec<String>,
    iota: Iota,
}

impl Ctx {
    fn with_trace<F, U>(&mut self, name: String, f: F) -> U
    where
        F: FnOnce(&mut Ctx) -> U,
    {
        self.trace.push(name);
        let result = f(self);
        self.trace.pop();
        result
    }
}

pub fn code(schema: Schema) -> Code {
    let mut ctx = Ctx {
        types: vec![],
        trace: vec![],
        iota: Iota::new(),
    };

    let root = match schema {
        Schema::Object(fields) => object(fields, &mut ctx),
        Schema::Array(ty) => {
            let inner = field_type(ty, &mut ctx);
            let root = ctx.iota.get();
            ctx.types.push(Type {
                id: root,
                ty: TypeType::Array(inner),
                trace: ctx.trace.clone(),
            });
            root
        }
    };

    Code {
        root,
        types: ctx.types,
    }
}

fn object(fields: Vec<Field>, ctx: &mut Ctx) -> usize {
    let mut obj_fields = vec![];

    for field in fields {
        obj_fields.push(ObjField {
            key: field.name.clone(),
            type_id: ctx.with_trace(field.name, |ctx| field_type(field.ty, ctx)),
        });
    }

    match ctx
        .types
        .iter()
        .filter_map(|Type { id, ty, trace: _ }| match ty {
            TypeType::Object(fields) => Some((id, fields)),
            _ => None,
        })
        .find(|(_, fields)| **fields == obj_fields)
    {
        Some((id, _)) => *id,
        None => {
            let id = ctx.iota.get();
            ctx.types.push(Type {
                id,
                ty: TypeType::Object(obj_fields),
                trace: ctx.trace.clone(),
            });
            id
        }
    }
}

fn field_type(ty: FieldType, ctx: &mut Ctx) -> usize {
    match ty {
        FieldType::String => match ctx.types.iter().find(|t| t.ty == TypeType::String) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(Type {
                    id,
                    ty: TypeType::String,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Integer => match ctx.types.iter().find(|t| t.ty == TypeType::Integer) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(Type {
                    id,
                    ty: TypeType::Integer,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Float => match ctx.types.iter().find(|t| t.ty == TypeType::Float) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(Type {
                    id,
                    ty: TypeType::Float,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Boolean => match ctx.types.iter().find(|t| t.ty == TypeType::Boolean) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(Type {
                    id,
                    ty: TypeType::Boolean,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Unknown => match ctx.types.iter().find(|t| t.ty == TypeType::Unknown) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(Type {
                    id,
                    ty: TypeType::Unknown,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Object(fields) => object(fields, ctx),
        FieldType::Union(types) => {
            let variant_ids: Vec<_> = types.into_iter().map(|ty| field_type(ty, ctx)).collect();
            match ctx
                .types
                .iter()
                .filter_map(|Type { id, ty, trace: _ }| match ty {
                    TypeType::Union(v) => Some((id, v)),
                    _ => None,
                })
                .find(|(_, v)| **v == variant_ids)
            {
                Some((id, _)) => *id,
                None => {
                    let id = ctx.iota.get();
                    ctx.types.push(Type {
                        id,
                        ty: TypeType::Union(variant_ids),
                        trace: ctx.trace.clone(),
                    });
                    id
                }
            }
        }
        FieldType::Array(ty) => {
            let inner_id = field_type(*ty, ctx);
            match ctx.types.iter().find(|t| t.ty == TypeType::Array(inner_id)) {
                Some(t) => t.id,
                None => {
                    let id = ctx.iota.get();
                    ctx.types.push(Type {
                        id,
                        ty: TypeType::Array(inner_id),
                        trace: ctx.trace.clone(),
                    });
                    id
                }
            }
        }
        FieldType::Optional(ty) => {
            let inner_id = field_type(*ty, ctx);
            match ctx
                .types
                .iter()
                .find(|t| t.ty == TypeType::Optional(inner_id))
            {
                Some(t) => t.id,
                None => {
                    let id = ctx.iota.get();
                    ctx.types.push(Type {
                        id,
                        ty: TypeType::Optional(inner_id),
                        trace: ctx.trace.clone(),
                    });
                    id
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::extract;
    use serde_json::Value;

    #[test]
    fn obj() {
        let json: Value = serde_json::from_str(
            r#"
            {
                "from": { "x": 0, "y": 0 },
                "nest": {
                    "from": { "a": "b", "c": "d" }
                },
                "to": { "x": 1, "y": 1 }
            }
            "#,
        )
        .unwrap();

        // println!("{:#?}", json);
        let schema = extract(json);
        let types = code(schema);
        println!("{:#?}", types);
    }

    #[test]
    fn arr() {
        let json: Value = serde_json::from_str(
            r#"
            [{"foo": ""}, {"bar": 2}]
            "#,
        )
        .unwrap();

        // println!("{:#?}", json);
        let schema = extract(json);
        let code = code(schema);
        println!("{:#?}", code);
    }
}
