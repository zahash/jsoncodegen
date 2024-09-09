use crate::{iota::Iota, schema::{Field, FieldType, Schema}};

#[derive(Debug)]
pub struct FlatSchema {
    pub root: usize,
    pub types: Vec<FlatType>,
}

#[derive(Debug)]
pub struct FlatType {
    pub id: usize,
    pub ty: FlatTypeKind,
    pub trace: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum FlatTypeKind {
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

struct Ctx {
    types: Vec<FlatType>,
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

pub fn flatten(schema: Schema) -> FlatSchema {
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
            ctx.types.push(FlatType {
                id: root,
                ty: FlatTypeKind::Array(inner),
                trace: ctx.trace.clone(),
            });
            root
        }
    };

    FlatSchema {
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
        .filter_map(|FlatType { id, ty, trace: _ }| match ty {
            FlatTypeKind::Object(fields) => Some((id, fields)),
            _ => None,
        })
        .find(|(_, fields)| **fields == obj_fields)
    {
        Some((id, _)) => *id,
        None => {
            let id = ctx.iota.get();
            ctx.types.push(FlatType {
                id,
                ty: FlatTypeKind::Object(obj_fields),
                trace: ctx.trace.clone(),
            });
            id
        }
    }
}

fn field_type(ty: FieldType, ctx: &mut Ctx) -> usize {
    match ty {
        FieldType::String => match ctx.types.iter().find(|t| t.ty == FlatTypeKind::String) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(FlatType {
                    id,
                    ty: FlatTypeKind::String,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Integer => match ctx.types.iter().find(|t| t.ty == FlatTypeKind::Integer) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(FlatType {
                    id,
                    ty: FlatTypeKind::Integer,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Float => match ctx.types.iter().find(|t| t.ty == FlatTypeKind::Float) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(FlatType {
                    id,
                    ty: FlatTypeKind::Float,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Boolean => match ctx.types.iter().find(|t| t.ty == FlatTypeKind::Boolean) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(FlatType {
                    id,
                    ty: FlatTypeKind::Boolean,
                    trace: ctx.trace.clone(),
                });
                id
            }
        },
        FieldType::Unknown => match ctx.types.iter().find(|t| t.ty == FlatTypeKind::Unknown) {
            Some(t) => t.id,
            None => {
                let id = ctx.iota.get();
                ctx.types.push(FlatType {
                    id,
                    ty: FlatTypeKind::Unknown,
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
                .filter_map(|FlatType { id, ty, trace: _ }| match ty {
                    FlatTypeKind::Union(v) => Some((id, v)),
                    _ => None,
                })
                .find(|(_, v)| **v == variant_ids)
            {
                Some((id, _)) => *id,
                None => {
                    let id = ctx.iota.get();
                    ctx.types.push(FlatType {
                        id,
                        ty: FlatTypeKind::Union(variant_ids),
                        trace: ctx.trace.clone(),
                    });
                    id
                }
            }
        }
        FieldType::Array(ty) => {
            let inner_id = field_type(*ty, ctx);
            match ctx.types.iter().find(|t| t.ty == FlatTypeKind::Array(inner_id)) {
                Some(t) => t.id,
                None => {
                    let id = ctx.iota.get();
                    ctx.types.push(FlatType {
                        id,
                        ty: FlatTypeKind::Array(inner_id),
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
                .find(|t| t.ty == FlatTypeKind::Optional(inner_id))
            {
                Some(t) => t.id,
                None => {
                    let id = ctx.iota.get();
                    ctx.types.push(FlatType {
                        id,
                        ty: FlatTypeKind::Optional(inner_id),
                        trace: ctx.trace.clone(),
                    });
                    id
                }
            }
        }
    }
}

pub struct TypeName {
    pub id: usize,
    pub name: String,
}

pub fn type_names<F>(code: &FlatSchema, mut format: F, default: String) -> Vec<TypeName>
where
    F: FnMut(&[String]) -> String,
{
    let mut taken = vec![];
    let mut type_names = vec![];

    for ty in &code.types {
        let name = unique_type_name(&ty.trace, &mut format, &mut taken, default.clone());
        type_names.push(TypeName { id: ty.id, name });
    }

    type_names
}

fn unique_type_name<F>(
    trace: &[String],
    mut format: F,
    taken: &mut Vec<String>,
    default: String,
) -> String
where
    F: FnMut(&[String]) -> String,
{
    for i in (0..trace.len()).rev() {
        let candidate = format(&trace[i..]);
        if !taken.contains(&candidate) {
            taken.push(candidate.clone());
            return candidate;
        }
    }

    let base_name = match trace.is_empty() {
        true => default,
        false => format(trace),
    };

    let mut count = 0;
    loop {
        let candidate = format!("{}{}", base_name, count);
        if !taken.contains(&candidate) {
            taken.push(candidate.clone());
            return candidate;
        }
        count += 1;
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
                "to": { "x": 1, "y": 1 },
                "g": [1, 2, 3, 4.0]
            }
            "#,
        )
        .unwrap();

        // println!("{:#?}", json);
        let schema = extract(json);
        let types = flatten(schema);
        println!("{:#?}", types);
    }

    #[test]
    fn arr() {
        let json: Value = serde_json::from_str(
            r#"
            [{"foo": ""}, {"bar": 2}, 1, 2, 3, 4, null]
            "#,
        )
        .unwrap();

        // println!("{:#?}", json);
        let schema = extract(json);
        let code = flatten(schema);
        println!("{:#?}", code);
    }
}
