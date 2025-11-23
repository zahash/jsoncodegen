use std::collections::BTreeMap;

use crate::{
    iota::Iota,
    schema::{Field, FieldType, Schema},
};

pub type TypeId = usize;

#[derive(Debug, Clone)]
pub struct TypeGraph {
    pub root: TypeId,
    pub nodes: BTreeMap<TypeId, TypeDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeDef {
    String,
    Integer,
    Float,
    Boolean,
    Unknown,
    Object(Vec<ObjectField>),
    Union(Vec<TypeId>),
    Array(TypeId),
    Optional(TypeId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectField {
    pub name: String,
    pub type_id: TypeId,
}

impl From<Schema> for TypeGraph {
    fn from(schema: Schema) -> Self {
        let builder = GraphBuilder::new();
        builder.process_schema(&schema)
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes: BTreeMap<TypeId, TypeDef>,
    cache: BTreeMap<TypeDef, TypeId>,
    iota: Iota,
}

impl GraphBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn process_schema(mut self, schema: &Schema) -> TypeGraph {
        let root_type_id = match schema {
            Schema::Object(fields) => self.process_fields(fields),
            Schema::Array(field_type) => {
                let inner_type_id = self.process_field_type(field_type);
                self.intern(TypeDef::Array(inner_type_id))
            }
        };

        TypeGraph {
            root: root_type_id,
            nodes: self.nodes,
        }
    }

    fn process_field_type(&mut self, field_type: &FieldType) -> TypeId {
        match field_type {
            FieldType::String => self.intern(TypeDef::String),
            FieldType::Integer => self.intern(TypeDef::Integer),
            FieldType::Float => self.intern(TypeDef::Float),
            FieldType::Boolean => self.intern(TypeDef::Boolean),
            FieldType::Unknown => self.intern(TypeDef::Unknown),
            FieldType::Object(fields) => self.process_fields(fields),
            FieldType::Union(field_types) => {
                let mut type_ids: Vec<TypeId> = field_types
                    .iter()
                    .map(|ty| self.process_field_type(ty))
                    .collect();
                type_ids.sort();
                self.intern(TypeDef::Union(type_ids))
            }
            FieldType::Array(inner_field_type) => {
                let inner_type_id = self.process_field_type(inner_field_type);
                self.intern(TypeDef::Array(inner_type_id))
            }
            FieldType::Optional(inner_field_type) => {
                let inner_type_id = self.process_field_type(inner_field_type);
                self.intern(TypeDef::Optional(inner_type_id))
            }
        }
    }

    fn process_fields(&mut self, fields: &[Field]) -> TypeId {
        let mut obj_fields = Vec::with_capacity(fields.len());
        for field in fields {
            obj_fields.push(ObjectField {
                name: field.name.clone(),
                type_id: self.process_field_type(&field.ty),
            });
        }
        obj_fields.sort_by(|a, b| a.name.cmp(&b.name));

        self.intern(TypeDef::Object(obj_fields))
    }

    fn intern(&mut self, type_def: TypeDef) -> TypeId {
        match self.cache.get(&type_def) {
            Some(type_id) => *type_id,
            None => {
                let type_id = self.iota.next();
                self.nodes.insert(type_id, type_def.clone());
                self.cache.insert(type_def, type_id);
                type_id
            }
        }
    }

    // fn optimize_recursive_types(&mut self) {
    //     #[derive(Debug)]
    //     struct ResolvedObject {
    //         type_ids: Vec<TypeId>,
    //         code_fields: Vec<ObjectField>,
    //     }

    //     let mut resolved_objects: Vec<ResolvedObject> = vec![];
    //     dbg!(&resolved_objects);

    //     for (type_id, type_def) in &self.nodes {
    //         dbg!(type_id, type_def);
    //         if let TypeDef::Object(code_fields) = type_def {
    //             let mut resolved = false;

    //             dbg!(&resolved_objects);
    //             for resolved_object in &mut resolved_objects {
    //                 dbg!(&resolved_object);
    //                 if let Some(resolved_code_fields) =
    //                     self.resolve_code_fields(&resolved_object.code_fields, code_fields)
    //                 {
    //                     dbg!(&resolved_code_fields);
    //                     *(&mut resolved_object.code_fields) = resolved_code_fields;
    //                     resolved_object.type_ids.push(*type_id);
    //                     dbg!(&resolved_object);
    //                     resolved = true;
    //                     break;
    //                 }
    //             }

    //             dbg!(resolved);
    //             if !resolved {
    //                 let resolved_object = ResolvedObject {
    //                     type_ids: vec![*type_id],
    //                     code_fields: code_fields.clone(),
    //                 };
    //                 dbg!(&resolved_object);
    //                 resolved_objects.push(resolved_object);
    //                 dbg!(&resolved_objects);
    //             }
    //         }
    //     }

    //     dbg!(resolved_objects);
    // }

    // fn resolve_code_fields(
    //     &self,
    //     xs: &[ObjectField],
    //     ys: &[ObjectField],
    // ) -> Option<Vec<ObjectField>> {
    //     dbg!(xs);
    //     dbg!(ys);
    //     if xs.len() != ys.len() {
    //         return None;
    //     }

    //     dbg!(xs
    //         .iter()
    //         .zip(ys)
    //         .map(|(x, y)| self.resolve_code_field(x, y))
    //         .collect())
    // }

    // fn resolve_code_field(&self, a: &ObjectField, b: &ObjectField) -> Option<ObjectField> {
    //     dbg!(a);
    //     dbg!(b);
    //     if a.name == b.name {
    //         if a.type_id == b.type_id {
    //             return dbg!(Some(a.clone()));
    //         }

    //         if matches!(self.nodes.get(&a.type_id), Some(TypeDef::Unknown)) {
    //             return dbg!(Some(ObjectField {
    //                 name: a.name.clone(),
    //                 type_id: b.type_id.clone(),
    //             }));
    //         }

    //         if matches!(self.nodes.get(&b.type_id), Some(TypeDef::Unknown)) {
    //             return dbg!(Some(ObjectField {
    //                 name: a.name.clone(),
    //                 type_id: a.type_id.clone(),
    //             }));
    //         }
    //     }

    //     dbg!(None)
    // }

    // fn signature<'cf>(code_fields: impl IntoIterator<Item = &'cf CodeField>) -> Vec<&'cf str> {
    //     let mut field_names: Vec<&str> = code_fields
    //         .into_iter()
    //         .map(|field| field.name.as_str())
    //         .collect();
    //     field_names.sort();
    //     field_names
    // }
}

#[derive(Default)]
struct TypeReducer {
    reduced_nodes: BTreeMap<TypeId, TypeDef>,
    cache: BTreeMap<TypeDef, TypeId>,
    remaps: Vec<(TypeId, TypeId)>, // original TypeGraph to reduced TypeGraph
    iota: Iota,
}

impl TypeReducer {
    fn new() -> Self {
        Self::default()
    }

    fn reduce(&mut self, type_graph: TypeGraph) {
        for (type_id, mut type_def) in type_graph.nodes {
            self.remap_type_def(&mut type_def);
            let reduced_type_id = self.reduce_type_def(type_def);
            self.remaps.push((type_id, reduced_type_id));
        }
    }

    fn reduce_type_def(&mut self, type_def: TypeDef) -> TypeId {
        match type_def {
            TypeDef::Object(object_fields) => {
                for (reduced_type_id, reduced_type_def) in &self.reduced_nodes {
                    if let TypeDef::Object(reduced_object_fields) = reduced_type_def {
                        if self.merge_object_fields(reduced_object_fields, &object_fields) {
                            return *reduced_type_id;
                        }
                    }
                }
                self.intern(TypeDef::Object(object_fields))
            }
            TypeDef::String
            | TypeDef::Integer
            | TypeDef::Float
            | TypeDef::Boolean
            | TypeDef::Unknown
            | TypeDef::Union(_)
            | TypeDef::Array(_)
            | TypeDef::Optional(_) => self.intern(type_def),
        }

        // self.intern(reduced_type_def)
    }

    fn merge_object_fields(
        &self,
        target: &mut Vec<ObjectField>,
        candidate: &[ObjectField],
    ) -> bool {
        if target.len() != candidate.len() {
            return false;
        }

        let mut merged: Vec<ObjectField> = Vec::with_capacity(target.len());
        for (target_field, candidate_field) in target.iter().zip(candidate.iter()) {
            let Some(merged_field) = self.merge_object_field(target_field, candidate_field) else {
                return false;
            };
            merged.push(merged_field);
        }

        *target = merged;
        true
    }

    fn merge_object_field(
        &self,
        target: &ObjectField,
        candidate: &ObjectField,
    ) -> Option<ObjectField> {
        if target.name != candidate.name {
            return None;
        } // names are same

        if target.type_id == candidate.type_id {
            return Some(target.clone());
        } // types are different

        todo!();
        // let a =

        None
    }

    fn intern(&mut self, type_def: TypeDef) -> TypeId {
        match self.cache.get(&type_def) {
            Some(type_id) => *type_id,
            None => {
                let type_id = self.iota.next();
                self.reduced_nodes.insert(type_id, type_def.clone());
                self.cache.insert(type_def, type_id);
                type_id
            }
        }
    }

    fn remap_type_def(&self, type_def: &mut TypeDef) {
        match type_def {
            TypeDef::Object(object_fields) => {
                for object_field in object_fields {
                    self.remap_type_id(&mut object_field.type_id);
                }
            }
            TypeDef::Union(type_ids) => {
                for type_id in type_ids {
                    self.remap_type_id(type_id);
                }
            }
            TypeDef::Array(type_id) | TypeDef::Optional(type_id) => self.remap_type_id(type_id),
            _ => { /* no-op */ }
        }
    }

    fn remap_type_id(&self, type_id: &mut TypeId) {
        for (old, new) in &self.remaps {
            if type_id == old {
                *type_id = *new;
            }
        }
    }
}

/*
impl CodeGraph {
    fn handle_recursive_types(&mut self) {
        fn signature(code_fields: &[CodeField]) -> Vec<&str> {
            let mut field_names: Vec<&str> = code_fields
                .iter()
                .map(|field| field.name.as_str())
                .collect();
            field_names.sort();
            field_names
        }

        fn clump<'o, 'cf>(
            objects: &'o [(TypeId, &'cf [CodeField])],
        ) -> Vec<Vec<&'o (TypeId, &'cf [CodeField])>> {
            fn can_clump(fields_a: &[CodeField], fields_b: &[CodeField]) -> bool {
                match (fields_a, fields_b) {
                    (
                        [fields_a_first, fields_a_rest @ ..],
                        [fields_b_first, fields_b_rest @ ..],
                    ) => {}
                    _ => {}
                }

                todo!()
            }

            todo!()

            // match objects {
            //     [first_obj, rest @ ..] => {
            //         let mut clumped = Cow::Borrowed(first_obj);
            //         for next_obj in rest {
            //             let (_, first_obj_code_fields) = first_obj;
            //             let (_, next_obj_code_fields) = next_obj;
            //             if can_clump(first_obj_code_fields, next_obj_code_fields) {
            //                 clumped.push(next_obj);
            //             }
            //         }

            //         let mut next_clump = clump(rest);
            //         next_clump.push(clumped);
            //         next_clump
            //     }
            //     _ => vec![],
            // }
        }

        let object_groups = {
            let mut type_groups: BTreeMap<Vec<&str>, Vec<(TypeId, &[CodeField])>> = BTreeMap::new();
            for (type_id, type_def) in &self.nodes {
                if let TypeDef::Object(code_fields) = type_def {
                    let signature = signature(code_fields);
                    type_groups
                        .entry(signature)
                        .or_default()
                        .push((*type_id, code_fields));
                }
            }
            type_groups
        };

        for objects in object_groups.values() {
            let clumped = clump(objects);
            todo!()
        }
    }

    fn apply_remapings(&mut self, map: BTreeMap<TypeId, TypeId>) {
        if map.is_empty() {
            return;
        }

        for type_def in self.nodes.values_mut() {
            match type_def {
                TypeDef::Object(code_fields) => {
                    for field in code_fields {
                        if let Some(target_type_id) = map.get(&field.type_id) {
                            field.type_id = *target_type_id;
                        }
                    }
                }
                TypeDef::Union(type_ids) => {
                    for type_id in type_ids {
                        if let Some(target_type_id) = map.get(type_id) {
                            *type_id = *target_type_id;
                        }
                    }
                }
                TypeDef::Array(type_id) | TypeDef::Optional(type_id) => {
                    if let Some(target_type_id) = map.get(type_id) {
                        *type_id = *target_type_id;
                    }
                }
                _ => { /* no-op */ }
            }
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn schema(json: &str) -> Schema {
        Schema::from(serde_json::from_str::<Value>(json).expect("invalid json string"))
    }

    #[test]
    fn test() {
        let json = r#"
        {
            "val": 1,
            "prev": {
                "val": 2,
                "prev": null,
                "next": null
            },
            "next": {
                "val": 3,
                "prev": null,
                "next": {
                    "val": 4,
                    "prev": null,
                    "next": null
                }
            }
        }
        "#;

        let schema = schema(json);
        println!("{}", schema);

        let type_graph = TypeGraph::from(schema);
        println!("{:#?}", type_graph);
    }
}
