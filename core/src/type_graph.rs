use std::collections::BTreeMap;

use jsoncodegen_iota::Iota;
use serde_json::Value;

use crate::schema::{Field, FieldType, Schema};

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

impl From<Value> for TypeGraph {
    fn from(json: Value) -> Self {
        let schema = Schema::from(json);
        TypeGraph::from(schema)
    }
}

impl From<Schema> for TypeGraph {
    fn from(schema: Schema) -> Self {
        let type_graph = GraphBuilder::new().process_schema(&schema);
        let reduced_type_graph = TypeReducer::new().reduce(type_graph);
        reduced_type_graph
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

    fn reduce(mut self, type_graph: TypeGraph) -> TypeGraph {
        for (type_id, mut type_def) in type_graph.nodes {
            self.remap_type_def(&mut type_def);
            let reduced_type_id = self.reduce_type_def(type_def);
            self.remaps.push((type_id, reduced_type_id));
        }

        let mut root = type_graph.root;
        self.remap_type_id(&mut root);

        TypeGraph {
            root,
            nodes: self.reduced_nodes,
        }
    }

    fn reduce_type_def(&mut self, type_def: TypeDef) -> TypeId {
        match type_def {
            TypeDef::Object(object_fields) => {
                let target_type_ids: Vec<TypeId> = self.reduced_nodes.keys().copied().collect();

                for target_type_id in target_type_ids {
                    if let Some(TypeDef::Object(target_object_fields)) =
                        self.reduced_nodes.get(&target_type_id).cloned()
                    {
                        if let Some(merged_object_fields) =
                            self.merge_object_fields(&target_object_fields, &object_fields)
                        {
                            self.reduced_nodes
                                .insert(target_type_id, TypeDef::Object(merged_object_fields));
                            return target_type_id;
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
    }

    fn merge_object_fields(
        &mut self,
        target: &[ObjectField],
        candidate: &[ObjectField],
    ) -> Option<Vec<ObjectField>> {
        if target.len() != candidate.len() {
            return None;
        }

        target
            .iter()
            .zip(candidate.iter())
            .map(|(target_field, candidate_field)| {
                self.merge_object_field(target_field, candidate_field)
            })
            // collecting Iterator<Item = Option<...>> as Option<Vec<...>>
            // yeilds Some(Vec<...>) only if every iterated item is Some(...)
            // else, None is yeilded
            .collect()
    }

    fn merge_object_field(
        &mut self,
        target: &ObjectField,
        candidate: &ObjectField,
    ) -> Option<ObjectField> {
        if target.name != candidate.name {
            return None;
        } // names are same from here

        if target.type_id == candidate.type_id {
            return Some(target.clone());
        } // types are different from here

        let target_type_def = self.reduced_nodes.get(&target.type_id)?;
        let candidate_type_def = self.reduced_nodes.get(&candidate.type_id)?;

        if let TypeDef::Unknown = target_type_def {
            return Some(ObjectField {
                name: candidate.name.clone(),
                type_id: self.intern(TypeDef::Optional(candidate.type_id)),
            });
        }

        if let TypeDef::Unknown = candidate_type_def {
            return Some(ObjectField {
                name: target.name.clone(),
                type_id: self.intern(TypeDef::Optional(target.type_id)),
            });
        }

        if let TypeDef::Optional(target_inner_type_id) = target_type_def {
            if target_inner_type_id == &candidate.type_id {
                return Some(target.clone());
            }
        }

        if let TypeDef::Optional(candidate_inner_type_id) = candidate_type_def {
            if candidate_inner_type_id == &target.type_id {
                return Some(candidate.clone());
            }
        }

        if let (TypeDef::Optional(target_inner_type_id), TypeDef::Optional(candidate_inner_type_id)) =
            (target_type_def, candidate_type_def)
            && target_inner_type_id != candidate_inner_type_id
        {
            // TODO
        }

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
