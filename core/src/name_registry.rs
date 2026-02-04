use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use crate::type_graph::{ObjectField, TypeDef, TypeGraph, TypeId};

pub struct NamePreference<FA, FO> {
    pub filter: FA,
    pub compare: FO,
}

impl<FA, FO> NamePreference<FA, FO> {
    pub fn apply(&mut self, names: &mut Vec<&str>)
    where
        FA: FnMut(&str) -> bool,
        FO: FnMut(&str, &str) -> Ordering,
    {
        names.retain(|name| (self.filter)(name));
        names.sort_by(|a, b| (self.compare)(a, b));
        names.dedup();
    }
}

#[derive(Debug)]
pub struct NameRegistry<'type_graph> {
    assigned_names: BTreeMap<TypeId, &'type_graph str>,
}

impl<'type_graph> NameRegistry<'type_graph> {
    pub fn build(
        type_graph: &'type_graph TypeGraph,
        mut pref: NamePreference<impl FnMut(&str) -> bool, impl FnMut(&str, &str) -> Ordering>,
    ) -> Self {
        let mut collected_names = NameCollector::collect(type_graph);
        collected_names
            .values_mut()
            .for_each(|names| pref.apply(names));
        Self {
            assigned_names: BipartiteMatcher::solve(collected_names),
        }
    }

    pub fn assigned_name(&self, type_id: TypeId) -> Option<&str> {
        self.assigned_names.get(&type_id).map(|name| name.as_ref())
    }
}

/// Maximum Bipartite Matching.
struct BipartiteMatcher<'type_graph> {
    graph: BTreeMap<TypeId, Vec<&'type_graph str>>,
    matched: BTreeMap<&'type_graph str, TypeId>,
    visited: BTreeSet<TypeId>,
}

impl<'a> BipartiteMatcher<'a> {
    fn solve(graph: BTreeMap<TypeId, Vec<&'a str>>) -> BTreeMap<TypeId, &'a str> {
        let mut matcher = Self {
            graph,
            matched: Default::default(),
            visited: Default::default(),
        };

        let type_ids: Vec<TypeId> = matcher.graph.keys().copied().collect();

        // We iterate through all TypeIds (Left nodes) to find a match for them.
        for type_id in type_ids {
            // Reset visited for the new search path
            matcher.visited.clear();
            matcher.try_match(type_id);
        }

        let mut result = BTreeMap::new();
        for (name, type_id) in &matcher.matched {
            result.insert(*type_id, *name);
        }
        result
    }

    /// Tries to find a match for `type_id` (TypeId).
    /// Returns true if `type_id` was successfully matched (possibly by displacing others).
    fn try_match(&mut self, type_id: TypeId) -> bool {
        // If we have already visited this node in the current augmenting path search, stop.
        if self.visited.contains(&type_id) {
            return false;
        }
        self.visited.insert(type_id);

        let Some(candidates) = self.graph.get(&type_id) else {
            // if the node has no candidates, it cannot be matched.
            return false;
        };

        // cloning vector of references (Vec<&str>) is cheap enough
        for name in candidates.clone() {
            // We can take `name` if:
            // 1. It is currently unassigned (None)
            // 2. OR the current owner (`owner`) can be moved to a different valid name
            let is_available = match self.matched.get(name) {
                None => true,
                Some(&owner) => self.try_match(owner),
            };

            if is_available {
                self.matched.insert(name, type_id);
                return true;
            }
        }
        false
    }
}

#[derive(Debug)]
struct NameCollector<'type_graph> {
    type_graph: &'type_graph TypeGraph,
    names: BTreeMap<TypeId, Vec<&'type_graph str>>,
    visited: BTreeSet<TypeId>,
}

impl<'type_graph> NameCollector<'type_graph> {
    fn collect(type_graph: &'type_graph TypeGraph) -> BTreeMap<TypeId, Vec<&'type_graph str>> {
        let mut name_collector = Self {
            type_graph,
            names: Default::default(),
            visited: Default::default(),
        };
        name_collector.process_type_id(type_graph.root);
        name_collector.names
    }

    fn process_type_id(&mut self, type_id: TypeId) {
        if self.visited.contains(&type_id) {
            return;
        }
        self.visited.insert(type_id);

        if let Some(type_def) = self.type_graph.type_def(type_id) {
            match type_def {
                TypeDef::Object(object_fields) => {
                    for object_field in object_fields {
                        self.process_object_field(object_field);
                    }
                }
                TypeDef::Union(inner_type_ids) => {
                    for inner_type_id in inner_type_ids {
                        self.process_type_id(*inner_type_id)
                    }
                }
                TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                    self.process_type_id(*inner_type_id)
                }
                _ => { /* no-op */ }
            }
        }
    }

    fn process_object_field(&mut self, object_field: &'type_graph ObjectField) {
        if let Some(object_field_type_def) = self.type_graph.type_def(object_field.type_id) {
            match object_field_type_def {
                TypeDef::Object(nested_object_fields) => {
                    let names = self.names.entry(object_field.type_id).or_default();
                    if !names.contains(&object_field.name.as_str()) {
                        names.push(&object_field.name);
                    }
                    for nested_object_field in nested_object_fields {
                        self.process_object_field(nested_object_field);
                    }
                }
                TypeDef::Union(inner_type_ids) => {
                    let names = self.names.entry(object_field.type_id).or_default();
                    if !names.contains(&object_field.name.as_str()) {
                        names.push(&object_field.name);
                    }
                    for inner_type_id in inner_type_ids {
                        self.process_type_id(*inner_type_id);
                    }
                }
                TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                    let inner_type_id = self.naming_target(*inner_type_id);
                    let names = self.names.entry(inner_type_id).or_default();
                    if !names.contains(&object_field.name.as_str()) {
                        names.push(&object_field.name);
                    }
                    self.process_type_id(inner_type_id);
                }
                _ => { /* no-op */ }
            }
        }
    }

    fn naming_target(&self, mut type_id: TypeId) -> TypeId {
        let mut visited = vec![];
        while !visited.contains(&type_id) {
            visited.push(type_id);
            if let Some(type_def) = self.type_graph.type_def(type_id) {
                if let TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) = type_def {
                    type_id = *inner_type_id;
                }
            }
        }
        type_id
    }
}
