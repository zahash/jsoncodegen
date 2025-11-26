use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::type_graph::{ObjectField, TypeDef, TypeGraph, TypeId};

#[derive(Debug)]
pub struct NameResolver<'type_graph> {
    names: HashMap<TypeId, Vec<&'type_graph str>>,
    visited: HashSet<TypeId>,
}

impl<'type_graph> NameResolver<'type_graph> {
    pub fn resolve(type_graph: &'type_graph TypeGraph) -> Self {
        let mut name_resolver = Self {
            names: HashMap::new(),
            visited: HashSet::new(),
        };
        name_resolver.resolve_type_id(type_graph.root, type_graph);
        name_resolver
    }

    pub fn names(&self, type_id: TypeId) -> &[&'type_graph str] {
        self.names
            .get(&type_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn resolve_type_id(&mut self, type_id: TypeId, type_graph: &'type_graph TypeGraph) {
        if self.visited.contains(&type_id) {
            return;
        }
        self.visited.insert(type_id);

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
                    let names = self.names.entry(object_field.type_id).or_default();
                    names.push(&object_field.name);
                    names.sort();
                    names.dedup();
                    for nested_object_field in nested_object_fields {
                        self.resolve_object_field(nested_object_field, type_graph);
                    }
                }
                TypeDef::Union(inner_type_ids) => {
                    let names = self.names.entry(object_field.type_id).or_default();
                    names.push(&object_field.name);
                    names.sort();
                    names.dedup();
                    for inner_type_id in inner_type_ids {
                        self.resolve_type_id(*inner_type_id, type_graph);
                    }
                }
                TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                    let names = self.names.entry(*inner_type_id).or_default();
                    names.push(&object_field.name);
                    names.sort();
                    names.dedup();
                    self.resolve_type_id(*inner_type_id, type_graph);
                }
                _ => { /* no-op */ }
            }
        }
    }
}

impl Display for NameResolver<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.names.iter();
        if let Some((type_id, names)) = iter.next() {
            write!(f, "{}:{:?}", type_id, names)?;
            for (type_id, names) in iter {
                write!(f, ";{}:{:?}", type_id, names)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        // TODO: this has infinite recursion
        // let json = r#"
        // {
        //     "val": 1,
        //     "prev": {
        //         "val": 2,
        //         "prev": null,
        //         "next": null
        //     },
        //     "next": {
        //         "val": 3,
        //         "prev": null,
        //         "next": {
        //             "val": 4,
        //             "prev": null,
        //             "next": null
        //         }
        //     }
        // }
        // "#;

        // let json = r#"
        //     {
        //         "id": "123",
        //         "name": "John Doe",
        //         "email": "john.doe@example.com",
        //         "address": {
        //             "street": "123 Main St",
        //             "city": "Anytown",
        //             "state": "CA",
        //             "zip": "12345"
        //         }
        //     }
        //     "#;

        let json = r#"
            {
                "user": {
                    "id": 123,
                    "name": "Alice",
                    "email": "alice@example.com",
                    "verified": true,
                    "address": {
                        "city": "London",
                        "zip": 40512
                    }
                },
                "cart": [
                    {
                        "sku": "SKU-123",
                        "qty": 2,
                        "price": 499.99,
                        "metadata": null
                    },
                    {
                        "sku": "SKU-999",
                        "qty": 1,
                        "price": 1299.50,
                        "metadata": { "color": "red" }
                    }
                ],
                "payment": null,
                "discount_codes": ["HOLIDAY", 2024, null]
            }
            "#;

        let json = serde_json::from_str::<serde_json::Value>(json).expect("invalid json");
        let type_graph = TypeGraph::from(json);
        let name_resolver = NameResolver::resolve(&type_graph);
        println!("type_graph={}", type_graph);
        println!("name_resolver={}", name_resolver);
    }
}
