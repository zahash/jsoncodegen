use std::collections::HashMap;

use crate::type_graph::{ObjectField, TypeDef, TypeGraph, TypeId};

#[derive(Debug)]
pub struct NameResolver<'type_graph> {
    names: HashMap<TypeId, &'type_graph str>,
}

impl<'type_graph> NameResolver<'type_graph> {
    pub fn resolve(type_graph: &'type_graph TypeGraph) -> Self {
        let mut name_resolver = Self {
            names: HashMap::new(),
        };
        name_resolver.resolve_type_id(type_graph.root, type_graph);
        name_resolver
    }

    pub fn get(&self, type_id: TypeId) -> Option<&'type_graph str> {
        self.names.get(&type_id).copied()
    }

    fn resolve_type_id(&mut self, type_id: TypeId, type_graph: &'type_graph TypeGraph) {
        if let Some(type_def) = type_graph.nodes.get(&type_id) {
            match type_def {
                TypeDef::Object(object_fields) => {
                    for object_field in object_fields {
                        self.resolve_object_field(object_field, type_graph);
                    }
                }
                TypeDef::Union(inner_type_ids) => {
                    for inner_type_id in inner_type_ids {
                        self.resolve_type_id(*inner_type_id, type_graph)
                    }
                }
                TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                    self.resolve_type_id(*inner_type_id, type_graph)
                }
                _ => { /* no-op */ }
            }
        }
    }

    fn resolve_object_field(
        &mut self,
        object_field: &'type_graph ObjectField,
        type_graph: &'type_graph TypeGraph,
    ) {
        if let Some(object_field_type_def) = type_graph.nodes.get(&object_field.type_id) {
            match object_field_type_def {
                TypeDef::Object(nested_object_fields) => {
                    if !self.names.contains_key(&object_field.type_id) {
                        self.names.insert(object_field.type_id, &object_field.name);
                        for nested_object_field in nested_object_fields {
                            self.resolve_object_field(nested_object_field, type_graph);
                        }
                    }
                }
                TypeDef::Union(inner_type_ids) => {
                    self.names.insert(object_field.type_id, &object_field.name);
                    for inner_type_id in inner_type_ids {
                        self.resolve_type_id(*inner_type_id, type_graph);
                    }
                }
                TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                    self.resolve_type_id(*inner_type_id, type_graph)
                }
                _ => { /* no-op */ }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let json = r#"
            {
                "id": "123",
                "name": "John Doe",
                "email": "john.doe@example.com",
                "address": {
                    "street": "123 Main St",
                    "city": "Anytown",
                    "state": "CA",
                    "zip": "12345"
                }
            }
            "#;

        let json = serde_json::from_str::<serde_json::Value>(json).expect("invalid json");
        let type_graph = dbg!(TypeGraph::from(json));
        dbg!(NameResolver::resolve(&type_graph));
    }
}
