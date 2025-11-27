use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
};

use crate::type_graph::{ObjectField, TypeDef, TypeGraph, TypeId};

#[derive(Debug)]
pub struct NameRegistry<'type_graph> {
    assigned_names: HashMap<TypeId, &'type_graph str>,
}

impl<'type_graph> NameRegistry<'type_graph> {
    pub fn build(type_graph: &'type_graph TypeGraph) -> Self {
        let name_resolver = NameResolver::resolve(type_graph);
        Self {
            assigned_names: name_resolver.assigned_names,
        }
    }

    pub fn assigned_name(&self, type_id: TypeId) -> Option<&str> {
        self.assigned_names.get(&type_id).map(|name| name.as_ref())
    }
}

#[derive(Debug, Default)]
struct NameResolver<'type_graph> {
    names: BTreeMap<TypeId, Vec<&'type_graph str>>,
    assigned_names: HashMap<TypeId, &'type_graph str>,
    visited: HashSet<TypeId>,
}

impl<'type_graph> NameResolver<'type_graph> {
    fn resolve(type_graph: &'type_graph TypeGraph) -> Self {
        let mut name_resolver = Self::default();
        name_resolver.resolve_type_id(type_graph.root, type_graph);
        name_resolver.assign_names();
        name_resolver
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

    /// Performs maximum assignment of unique names to ids using a DFS-based
    /// augmenting-path algorithm (Kuhn). Result is stored in `self.assigned_names`.
    fn assign_names(&mut self) {
        // Deterministic left side order (ids)
        let ids_order: Vec<usize> = self.names.keys().copied().collect();
        let n_ids = ids_order.len();

        // Map each unique &'type_graph str to an index on the "right" side
        let mut name_to_index: HashMap<&'type_graph str, usize> = HashMap::new();
        let mut unique_names: Vec<&'type_graph str> = Vec::new();

        // adjacency: for each left-index (position in ids_order) store list of right-indexes
        let mut id_to_name_indices: Vec<Vec<usize>> = vec![Vec::new(); n_ids];

        for (left_pos, id) in ids_order.iter().enumerate() {
            if let Some(candidates) = self.names.get(id) {
                let mut seen_local: HashSet<&'type_graph str> = HashSet::new();
                for &name in candidates {
                    // dedupe duplicates inside the same id's list
                    if !seen_local.insert(name) {
                        continue;
                    }
                    let idx = *name_to_index.entry(name).or_insert_with(|| {
                        unique_names.push(name);
                        unique_names.len() - 1
                    });
                    id_to_name_indices[left_pos].push(idx);
                }
            }
        }

        let n_names = unique_names.len();
        // For each name (right node) store Option<left_pos> it's matched to
        let mut name_matched_to: Vec<Option<usize>> = vec![None; n_names];

        // recursive DFS function to find augmenting path for left node `u`
        fn try_assign(
            u: usize,
            id_to_name_indices: &Vec<Vec<usize>>,
            visited: &mut Vec<bool>,
            name_matched_to: &mut Vec<Option<usize>>,
        ) -> bool {
            for &name_idx in &id_to_name_indices[u] {
                if visited[name_idx] {
                    continue;
                }
                visited[name_idx] = true;
                if name_matched_to[name_idx].is_none()
                    || try_assign(
                        name_matched_to[name_idx].unwrap(), // TODO: avoid unwrap
                        id_to_name_indices,
                        visited,
                        name_matched_to,
                    )
                {
                    // match name -> u
                    name_matched_to[name_idx] = Some(u);
                    return true;
                }
            }
            false
        }

        // Try to find a match for each left node
        for u in 0..n_ids {
            let mut visited = vec![false; n_names];
            try_assign(u, &id_to_name_indices, &mut visited, &mut name_matched_to);
        }

        // Build assigned_names map from matches
        self.assigned_names.clear();
        for (name_idx, opt_left_pos) in name_matched_to.into_iter().enumerate() {
            if let Some(left_pos) = opt_left_pos {
                let id = ids_order[left_pos];
                let name = unique_names[name_idx];
                self.assigned_names.insert(id, name);
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

        // let json = r#"
        //     {
        //         "user": {
        //             "id": 123,
        //             "name": "Alice",
        //             "email": "alice@example.com",
        //             "verified": true,
        //             "address": {
        //                 "city": "London",
        //                 "zip": 40512
        //             }
        //         },
        //         "cart": [
        //             {
        //                 "sku": "SKU-123",
        //                 "qty": 2,
        //                 "price": 499.99,
        //                 "metadata": null
        //             },
        //             {
        //                 "sku": "SKU-999",
        //                 "qty": 1,
        //                 "price": 1299.50,
        //                 "metadata": { "color": "red" }
        //             }
        //         ],
        //         "payment": null,
        //         "discount_codes": ["HOLIDAY", 2024, null]
        //     }
        //     "#;

        let json = serde_json::from_str::<serde_json::Value>(json).expect("invalid json");
        let type_graph = TypeGraph::from(json);
        let name_registry = NameRegistry::build(&type_graph);
        println!("type_graph={}", type_graph);
        println!("name_registry={:?}", name_registry);
    }
}
