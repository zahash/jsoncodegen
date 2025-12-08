use std::{
    collections::{BTreeMap, HashSet},
    fmt::Display,
};

use jsoncodegen_iota::Iota;
use serde_json::Value;

use crate::{
    name_registry::NameRegistry,
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
    Unknown,
    Null,
    Boolean,
    Integer,
    Float,
    String,
    Array(TypeId),
    Object(Vec<ObjectField>),
    Optional(TypeId),
    Union(Vec<TypeId>),
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
        let type_graph: TypeGraph = GraphBuilder::build(schema);
        let reduced_type_graph: TypeGraph = TypeReducer::reduce(type_graph);
        reduced_type_graph
    }
}

/// Canonicalize the type definition to ensure structural equality.
/// This allows deduplication: semantically identical types with different
/// orderings (e.g., `Union([1,2])` vs `Union([2,1]))` are treated as the same type.
fn canonicalize(type_def: &mut TypeDef) {
    if let TypeDef::Object(fields) = type_def {
        fields.sort_by(|a, b| a.name.cmp(&b.name));
    }
    if let TypeDef::Union(type_ids) = type_def {
        type_ids.sort();
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes: BTreeMap<TypeId, TypeDef>,
    cache: BTreeMap<TypeDef, TypeId>,
    iota: Iota,
}

impl GraphBuilder {
    fn build(schema: Schema) -> TypeGraph {
        let mut builder = GraphBuilder::default();

        let root_type_id = match schema {
            Schema::Object(fields) => builder.process_fields(fields),
            Schema::Array(field_type) => {
                let inner_type_id = builder.process_field_type(field_type);
                builder.intern(TypeDef::Array(inner_type_id))
            }
        };

        TypeGraph {
            root: root_type_id,
            nodes: builder.nodes,
        }
    }

    fn process_field_type(&mut self, field_type: FieldType) -> TypeId {
        match field_type {
            FieldType::String => self.intern(TypeDef::String),
            FieldType::Integer => self.intern(TypeDef::Integer),
            FieldType::Float => self.intern(TypeDef::Float),
            FieldType::Boolean => self.intern(TypeDef::Boolean),
            FieldType::Null => self.intern(TypeDef::Null),
            FieldType::Unknown => self.intern(TypeDef::Unknown),
            FieldType::Object(fields) => self.process_fields(fields),
            FieldType::Union(field_types) => {
                let type_ids: Vec<TypeId> = field_types
                    .into_iter()
                    .map(|ty| self.process_field_type(ty))
                    .collect();
                self.intern(TypeDef::Union(type_ids))
            }
            FieldType::Array(inner_field_type) => {
                let inner_type_id = self.process_field_type(*inner_field_type);
                self.intern(TypeDef::Array(inner_type_id))
            }
            FieldType::Optional(inner_field_type) => {
                let inner_type_id = self.process_field_type(*inner_field_type);
                self.intern(TypeDef::Optional(inner_type_id))
            }
        }
    }

    fn process_fields(&mut self, fields: Vec<Field>) -> TypeId {
        let obj_fields: Vec<ObjectField> = fields
            .into_iter()
            .map(|field| ObjectField {
                name: field.name,
                type_id: self.process_field_type(field.ty),
            })
            .collect();

        self.intern(TypeDef::Object(obj_fields))
    }

    fn intern(&mut self, mut type_def: TypeDef) -> TypeId {
        canonicalize(&mut type_def);

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
    fn reduce(type_graph: TypeGraph) -> TypeGraph {
        let mut reducer = TypeReducer::default();

        for (type_id, mut type_def) in type_graph.nodes {
            reducer.remap_type_def(&mut type_def);
            let reduced_type_id = reducer.reduce_type_def(type_def);
            reducer.remaps.push((type_id, reduced_type_id));
        }

        let mut root = type_graph.root;
        reducer.remap_type_id(&mut root);

        TypeGraph {
            root,
            nodes: reducer.reduced_nodes,
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
            | TypeDef::Null
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

    /*
    TODO:
    {
        "name": "Root",
        "children": [
            {
                "name": "Child1",
                "children": []
            }
        ]
    }

    it generates this right now

    use serde::{Serialize, Deserialize};

    pub type ROOT = Type5;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Type3 {
        pub children: Vec<serde_json::Value>,
        pub name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Type5 {
        pub children: Vec<Type3>,
        pub name: String,
    }

    but it should have generated this

    use serde::{Serialize, Deserialize};

    pub type ROOT = Type3;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Type3 {
        pub children: Vec<Type3>,
        pub name: String,
    }
    */
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

        // Unknown represents lack of information, so it adopts the concrete type
        if let TypeDef::Unknown = target_type_def {
            return Some(candidate.clone());
        }

        if let TypeDef::Unknown = candidate_type_def {
            return Some(target.clone());
        }

        // Null represents an explicit null value, so it creates Optional
        if let TypeDef::Null = target_type_def {
            return Some(ObjectField {
                name: candidate.name.clone(),
                type_id: self.intern(TypeDef::Optional(candidate.type_id)),
            });
        }

        if let TypeDef::Null = candidate_type_def {
            return Some(ObjectField {
                name: target.name.clone(),
                type_id: self.intern(TypeDef::Optional(target.type_id)),
            });
        }

        if let TypeDef::Optional(target_inner_type_id) = target_type_def
            && target_inner_type_id == &candidate.type_id
        {
            return Some(target.clone());
        }

        if let TypeDef::Optional(candidate_inner_type_id) = candidate_type_def
            && candidate_inner_type_id == &target.type_id
        {
            return Some(candidate.clone());
        }

        if let (TypeDef::Optional(target_inner_type_id), TypeDef::Optional(candidate_inner_type_id)) =
            (target_type_def, candidate_type_def)
            && target_inner_type_id != candidate_inner_type_id
        {
            // TODO
        }

        None
    }

    fn intern(&mut self, mut type_def: TypeDef) -> TypeId {
        canonicalize(&mut type_def);

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

struct CanonicalView<'type_graph> {
    type_graph: &'type_graph TypeGraph,
    name_registry: NameRegistry<'type_graph>,
}

impl<'type_graph> From<&'type_graph TypeGraph> for CanonicalView<'type_graph> {
    fn from(type_graph: &'type_graph TypeGraph) -> Self {
        Self {
            type_graph,
            name_registry: NameRegistry::build(type_graph),
        }
    }
}

impl<'type_graph> CanonicalView<'type_graph> {
    /// Format a type body for `type_id`. This does NOT print the label
    /// (name or `#id`) for the node itself — the caller prints that.
    fn fmt_type(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        type_id: TypeId,
        visited: &mut HashSet<TypeId>,
    ) -> std::fmt::Result {
        if visited.contains(&type_id) {
            return match self.name_registry.assigned_name(type_id) {
                Some(name) => write!(f, "{}", name),
                None => write!(f, "#{}", type_id),
            };
        }
        visited.insert(type_id);

        if let Some(type_def) = self.type_graph.nodes.get(&type_id) {
            match type_def {
                TypeDef::String => write!(f, "str")?,
                TypeDef::Integer => write!(f, "int")?,
                TypeDef::Float => write!(f, "float")?,
                TypeDef::Boolean => write!(f, "bool")?,
                TypeDef::Null => write!(f, "null")?,
                TypeDef::Unknown => write!(f, "unknown")?,
                TypeDef::Object(object_fields) => {
                    write!(f, "{{")?;
                    let mut iter = object_fields.iter();
                    if let Some(object_field) = iter.next() {
                        write!(f, "{}:", object_field.name)?;
                        self.fmt_type(f, object_field.type_id, visited)?;

                        for object_field in iter {
                            write!(f, ", {}:", object_field.name)?;
                            self.fmt_type(f, object_field.type_id, visited)?;
                        }
                    }
                    write!(f, "}}")?;
                }
                TypeDef::Array(inner_type_id) => {
                    write!(f, "[")?;
                    self.fmt_type(f, *inner_type_id, visited)?;
                    write!(f, "]")?;
                }
                TypeDef::Optional(inner_type_id) => {
                    self.fmt_type(f, *inner_type_id, visited)?;
                    write!(f, "?")?;
                }
                TypeDef::Union(inner_type_ids) => {
                    for inner_type_id in inner_type_ids {
                        write!(f, "|")?;
                        self.fmt_type(f, *inner_type_id, visited)?;
                    }
                    write!(f, "|")?;
                }
            }
        }

        // Remove from stack so siblings can still visit it
        // (we only want to detect cycles on the current path)
        visited.remove(&type_id);
        Ok(())
    }
}

impl Display for CanonicalView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print the label for the root (name or fallback to id)
        match self.name_registry.assigned_name(self.type_graph.root) {
            Some(name) => write!(f, "{}:", name),
            None => write!(f, "#{}:", self.type_graph.root),
        }?;

        // Then print the body of the root type
        let mut visited = HashSet::new();
        self.fmt_type(f, self.type_graph.root, &mut visited)
    }
}

impl Display for TypeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", CanonicalView::from(self))
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
        println!("{:?}", type_graph);
        println!("{}", type_graph);
    }
}
