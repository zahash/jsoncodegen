//! # Type Graph
//!
//! Type graph representation and reduction for efficient type management.
//!
//! This module provides a graph-based type system that converts schema representations
//! into a deduplicated, structurally-reduced type graph.
//!
//! The type graph enables:
//! - **Type Deduplication**: Structurally identical types share the same node
//! - **Type Reduction**: Structurally compatible objects are merged into unified types
//! - **Efficient Reference**: Types are referenced by ID, enabling recursive structures
//! - **Name Generation**: Supports generating type names via [`NameRegistry`]
//!
//! ## Core Data Structures
//!
//! ### [`TypeGraph`]
//!
//! It consists of:
//! - **Root**: The entry point TypeId representing the top-level schema
//! - **Nodes**: A map from TypeId to TypeDef, containing all type definitions
//!
//! Types are identified by [`TypeId`] (an alias for `usize`) and reference each other
//! through these IDs, allowing recursive and cyclic type relationships.
//!
//! ### [`TypeDef`]
//!
//! It mirrors the [`FieldType`] hierarchy from schema.rs:
//! - Primitives: `Unknown`, `Null`, `Boolean`, `Integer`, `Float`, `String`
//! - Containers: Array([TypeId]), Object(Vec<[ObjectField]>)
//! - Modifiers: Optional([TypeId]), Union(Vec<[TypeId]>)
//!
//! ## Two-Phase Construction
//!
//! Type graph construction happens in two phases:
//!
//! #### 1. Building ([`GraphBuilder`])
//!
//! Converts [`Schema`] → [`TypeGraph`] with deduplication:
//! - Traverses schema structure recursively
//! - Interns each type into a cache (BTreeMap<[TypeDef], [TypeId]>)
//! - Reuses existing [TypeId]s for structurally identical types
//! - Canonicalizes types before interning (sorts fields/union members)
//!
//! #### 2. Reduction ([`TypeReducer`])
//!
//! Merges structurally compatible Object types (primitives/Arrays/Unions/Optionals pass through unchanged):
//! - Processes types sequentially, remapping TypeIds to reduced equivalents
//! - For Objects: attempts merge with existing Objects in reduced graph
//! - Merge succeeds if: same field count, matching field names (after sort), all field pairs mergeable
//! - On success: updates existing Object in-place, reuses its TypeId
//! - On failure: interns as new Object
//! - Field merging handles: `Unknown+T→T`, `Null+T→Optional<T>`, `Optional(T)+T→Optional(T)`
//! - Incompatible field types (e.g., `Integer` vs `String`) prevent merging entirely
//!
//! ## Type Deduplication
//!
//! Deduplication ensures that structurally identical types share the same [`TypeId`]:
//!
//! - [`Schema`]: [{x:int}, {x:int}]  →  TypeGraph with single Object(x:int) node
//! - [`Schema`]: [{a:str}, {a:str}]  →  TypeGraph with single Object(a:str) node
//!
//! This is achieved through:
//! - Canonicalization: Sorting fields alphabetically and union members by ID
//! - Caching: Using BTreeMap<[TypeDef], [TypeId]> for O(log n) lookup
//! - Structural equality: PartialEq/Eq derived for [`TypeDef`] enables exact matching
//!
//! ## Type Reduction
//!
//! Reduction merges Objects with compatible structure. Only `Object` types are reduced; all others pass through.
//!
//! **Successful merge** (Null creates Optional):
//! ```text
//! Object([{name:"id", type_id:0}])  // 0 = Null
//! Object([{name:"id", type_id:1}])  // 1 = Integer
//! → Object([{name:"id", type_id:2}]) // 2 = Optional(Integer)
//! ```
//!
//! **Failed merge** (incompatible types remain separate):
//! ```text
//! Object([{name:"id", type_id:0}])  // 0 = Integer
//! Object([{name:"id", type_id:1}])  // 1 = String
//! → Both Objects remain distinct (no merge, no Union creation)
//! ```
//!
//! **Merge requirements**: Same field count, matching names (sorted), all fields individually mergeable via special rules.
//!
//! **Note**: Unions are created during schema inference (schema.rs), not during reduction.
//!
//! ## Canonicalization
//!
//! Canonicalization ensures deterministic representation:
//! - Object fields are sorted alphabetically by name
//! - Union members are sorted by TypeId
//! - Performed automatically in `intern()` before caching
//!
//! This guarantees that structurally equivalent types are recognized as identical
//! regardless of the order they were encountered.
//!
//! ## Example Workflow
//!
//! Linked list with nullable `next`/`prev` pointers demonstrating recursive types and reduction:
//!
//! ```text
//! JSON: [
//!   { "val": 1, "next": null, "prev": null },
//!   { "val": 1, "next": { "val": 2, "next": null, "prev": null }, "prev": null },
//!   { "val": 1, "next": null, "prev": { "val": 2, "next": null, "prev": null } }
//! ]
//!
//! 1. Schema Inference:
//! Array(
//!   Object([
//!     Field { name: "val", ty: Integer },
//!     Field {
//!       name: "next",
//!       ty: Optional(
//!         Object([
//!           Field { name: "val",  ty: Integer },
//!           Field { name: "next", ty: Null },
//!           Field { name: "prev", ty: Null },
//!         ]),
//!       ),
//!     },
//!     Field {
//!       name: "prev",
//!       ty: Optional(
//!         Object([
//!           Field { name: "val",  ty: Integer },
//!           Field { name: "next", ty: Null },
//!           Field { name: "prev", ty: Null },
//!         ]),
//!       ),
//!     },
//!   ]),
//! )
//!
//! 3. Graph Building (with deduplication):
//!   TypeGraph {
//!     root: 4,
//!     nodes: {
//!       0: Null,
//!       1: Integer,
//!       2: Object([
//!         ObjectField{name:"val",  type_id:1},
//!         ObjectField{name:"next", type_id:3},
//!         ObjectField{name:"prev", type_id:3}
//!       ]),
//!       3: Optional(2),  // Recursive reference: Optional<Node>
//!       4: Array(2)
//!     }
//!   }
//!
//! Reduction:
//!   All three objects reduced to single Object type
//!
//! Canonical View: #4:[{next:next?, prev:next?, val:int}]
//!   - Recursive self-reference: `next` and `prev` both point to Optional<Node>
//!   - Compact representation of potentially infinite linked list structure
//! ```
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Display,
};

use jsoncodegen_iota::Iota;
use serde_json::Value;

use crate::{
    name_registry::{NamePreference, NameRegistry},
    schema::{Field, FieldType, Schema},
};

/// Type identifier for referencing types within the graph.
pub type TypeId = usize;

/// Type graph: root [`TypeId`] + map of [`TypeId`] to [`TypeDef`].
///
/// Types reference each other via TypeId, enabling recursive structures.
#[derive(Debug, Clone)]
pub struct TypeGraph {
    pub root: TypeId,
    nodes: BTreeMap<TypeId, TypeDef>,
}

/// Type definition node. Mirrors FieldType but uses [`TypeId`] references.
///
/// Implements Ord for BTreeMap caching (interning).
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

/// Named field in object type.
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

/// Builds TypeGraph from [`Schema`]: build phase + reduction phase.
impl From<Schema> for TypeGraph {
    fn from(schema: Schema) -> Self {
        let type_graph: TypeGraph = GraphBuilder::build(schema);
        let reduced_type_graph: TypeGraph = TypeReducer::reduce(type_graph);
        reduced_type_graph
    }
}

impl TypeGraph {
    pub fn type_def(&self, type_id: TypeId) -> Option<&TypeDef> {
        self.nodes.get(&type_id)
    }
}

pub struct TypeGraphIter<'type_graph> {
    type_graph: &'type_graph TypeGraph,
    frontier: VecDeque<TypeId>,
    visited: BTreeSet<TypeId>,
}

impl<'type_graph> TypeGraphIter<'type_graph> {
    fn new(type_graph: &'type_graph TypeGraph) -> Self {
        Self {
            type_graph,
            frontier: VecDeque::from([type_graph.root]),
            visited: BTreeSet::new(),
        }
    }
}

impl<'type_graph> Iterator for TypeGraphIter<'type_graph> {
    type Item = (TypeId, &'type_graph TypeDef);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(type_id) = self.frontier.pop_front() {
            if self.visited.insert(type_id) {
                if let Some(type_def) = self.type_graph.type_def(type_id) {
                    match type_def {
                        TypeDef::Object(object_fields) => self
                            .frontier
                            .extend(object_fields.iter().map(|field| field.type_id)),
                        TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                            self.frontier.push_back(*inner_type_id)
                        }
                        TypeDef::Union(inner_type_ids) => self.frontier.extend(inner_type_ids),
                        _ => { /* no-op */ }
                    };
                    return Some((type_id, type_def));
                }
            }
        }

        None
    }
}

impl<'type_graph> IntoIterator for &'type_graph TypeGraph {
    type Item = (TypeId, &'type_graph TypeDef);
    type IntoIter = TypeGraphIter<'type_graph>;

    fn into_iter(self) -> Self::IntoIter {
        TypeGraphIter::new(self)
    }
}

/// Canonicalizes TypeDef: sorts Object fields by name, Union members by inner TypeDef.
///
/// Ensures structural equality for deduplication of semantically identical types.
/// e.g., `Union([1,2])` and `Union([2,1])` are treated as the same type.
fn canonicalize(type_def: &mut TypeDef, nodes: &BTreeMap<TypeId, TypeDef>) {
    if let TypeDef::Object(fields) = type_def {
        fields.sort_by(|a, b| a.name.cmp(&b.name));
    }

    if let TypeDef::Union(inner_type_ids) = type_def {
        inner_type_ids.sort_by_key(|id| match nodes.get(id) {
            Some(inner_type_def) => match inner_type_def {
                TypeDef::Unknown => 0,
                TypeDef::Null => 1,
                TypeDef::Boolean => 2, // Simplest primitive type
                TypeDef::Integer => 3, // Numeric types ordered by specificity
                TypeDef::Float => 4,   // More general numeric type
                TypeDef::String => 5,
                TypeDef::Array(_) => 6, // Collection types before complex structures
                TypeDef::Object(_) => 7, // Complex structured type
                TypeDef::Optional(_) => 8, // Wrapper types that modify other types
                TypeDef::Union(_) => 9, // Most complex - union of multiple types
            },
            None => usize::MAX, // Unknown types go last. usually unreachable!
        });
    }
}

/// Builds TypeGraph from Schema with type deduplication via canonicalization + caching.
#[derive(Default)]
struct GraphBuilder {
    nodes: BTreeMap<TypeId, TypeDef>,
    cache: BTreeMap<TypeDef, TypeId>,
    iota: Iota,
}

impl GraphBuilder {
    /// Builds Canonicalized TypeGraph from Schema by processing root and all nested types.
    ///
    /// see [`canonicalize`] for details on canonicalization process.
    fn build(schema: Schema) -> TypeGraph {
        let mut builder = GraphBuilder::default();

        let root_type_id = builder.process_field_type(schema.ty);

        TypeGraph {
            root: root_type_id,
            nodes: builder.nodes,
        }
    }

    /// Converts FieldType to TypeId, recursively processing nested types.
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

    /// Converts Fields to Object TypeId.
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

    /// Interns TypeDef: canonicalize, check cache, return existing or create new TypeId.
    fn intern(&mut self, mut type_def: TypeDef) -> TypeId {
        canonicalize(&mut type_def, &self.nodes);

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

/// Reduces only Object types by merging compatible instances. Other types pass through unchanged.
///
/// Algorithm: For each Object, try merging with existing Objects (same fields, mergeable types).
/// On success: update existing Object in-place. On failure: intern as new Object.
///
/// see [`TypeReducer::merge_object_field`] for detailed object field merge rules.
#[derive(Default)]
struct TypeReducer {
    reduced_nodes: BTreeMap<TypeId, TypeDef>,
    cache: BTreeMap<TypeDef, TypeId>,
    remaps: Vec<(TypeId, TypeId)>, // original TypeGraph to reduced TypeGraph
    iota: Iota,
}

impl TypeReducer {
    /// Reduces TypeGraph: processes each type, remaps TypeIds, merges compatible Objects.
    /// NOTE: TypeGraph must already be canonicalized (from GraphBuilder phase)
    ///
    /// Algorithm:
    /// 1. For each type: remap contained TypeIds using remaps table (points to reduced types)
    /// 2. Reduce type: Objects try merging with existing; others just intern
    /// 3. Record mapping: original TypeId → reduced TypeId
    /// 4. Remap root TypeId and return reduced graph
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

    /// Reduces a single type. Objects: iterate existing Objects, try merge, update in-place on success.
    /// Non-Objects: intern directly (deduplication only, no reduction).
    fn reduce_type_def(&mut self, type_def: TypeDef) -> TypeId {
        match type_def {
            TypeDef::Object(object_fields) => {
                let target_type_ids: Vec<TypeId> = self.reduced_nodes.keys().copied().collect();

                for target_type_id in target_type_ids {
                    if let Some(TypeDef::Object(target_object_fields)) =
                        self.reduced_nodes.get(&target_type_id)
                    {
                        let target_object_fields = target_object_fields.clone();
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

    /// Merges field lists: checks length equality, sorts both by name, zips and merges pairwise.
    /// Returns Some(merged) only if all field pairs successfully merge; else None.
    /// NOTE: it is assumed that both target and candidate object fields are canonicalized (sorted by name).
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

    /// Merges two fields with same name. Returns None if names differ or types incompatible.
    ///
    /// Rules:
    /// - `T + T → T` (identical TypeIds)
    /// - `Unknown + T → T` (Unknown adopts concrete type)
    /// - `Null + T → Optional(T)` (creates Optional, unless T already Optional)
    /// - `Null + Optional(T) → Optional(T)` (already optional, no double-wrap)
    /// - `Optional(T) + T → Optional(T)` (already optional)
    /// - `T1 + T2` (incompatible) → None (no Union creation; causes merge failure)
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
            // If candidate is already Optional, don't double-wrap
            if matches!(candidate_type_def, TypeDef::Optional(_)) {
                return Some(candidate.clone());
            }
            return Some(ObjectField {
                name: candidate.name.clone(),
                type_id: self.intern(TypeDef::Optional(candidate.type_id)),
            });
        }

        if let TypeDef::Null = candidate_type_def {
            // If target is already Optional, don't double-wrap
            if matches!(target_type_def, TypeDef::Optional(_)) {
                return Some(target.clone());
            }
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

        None
    }

    fn intern(&mut self, mut type_def: TypeDef) -> TypeId {
        canonicalize(&mut type_def, &self.reduced_nodes);

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

    /// Remaps all TypeId references in TypeDef from original to reduced IDs.
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

/// TypeGraph view with generated type names for display.
struct CanonicalView<'type_graph> {
    type_graph: &'type_graph TypeGraph,
    name_registry: NameRegistry<'type_graph>,
}

impl<'type_graph> From<&'type_graph TypeGraph> for CanonicalView<'type_graph> {
    fn from(type_graph: &'type_graph TypeGraph) -> Self {
        Self {
            type_graph,
            name_registry: NameRegistry::build(
                type_graph,
                NamePreference {
                    filter: |_: &str| true,
                    compare: |a: &str, b: &str| a.cmp(b),
                },
            ),
        }
    }
}

impl<'type_graph> CanonicalView<'type_graph> {
    /// Formats type recursively with cycle detection (visited set prevents infinite recursion).
    fn fmt_type(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        type_id: TypeId,
        visited: &mut BTreeSet<TypeId>,
    ) -> std::fmt::Result {
        if visited.contains(&type_id) {
            return match self.name_registry.assigned_name(type_id) {
                Some(name) => write!(f, "{}", name),
                None => write!(f, "#{}", type_id),
            };
        }
        visited.insert(type_id);

        if let Some(type_def) = self.type_graph.type_def(type_id) {
            match type_def {
                TypeDef::String => write!(f, "str")?,
                TypeDef::Integer => write!(f, "int")?,
                TypeDef::Float => write!(f, "float")?,
                TypeDef::Boolean => write!(f, "bool")?,
                TypeDef::Null => write!(f, "null")?,
                TypeDef::Unknown => write!(f, "unknown")?,
                TypeDef::Object(object_fields) => {
                    write!(f, "{{")?;
                    if let [first, rest @ ..] = object_fields.as_slice() {
                        write!(f, "{}:", first.name)?;
                        self.fmt_type(f, first.type_id, visited)?;

                        for object_field in rest {
                            write!(f, ",{}:", object_field.name)?;
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
            Some(name) => write!(f, "{};", name),
            None => write!(f, "#{};", self.type_graph.root),
        }?;

        // Then print the body of the root type
        let mut visited = BTreeSet::new();
        self.fmt_type(f, self.type_graph.root, &mut visited)
    }
}

impl Display for TypeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", CanonicalView::from(self))
    }
}
