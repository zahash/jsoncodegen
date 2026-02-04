use std::{io, iter};

use convert_case::{Case, Casing};
use jsoncodegen::{
    name_registry::{NamePreference, NameRegistry},
    type_graph::{TypeDef, TypeGraph, TypeId},
};

pub fn codegen(json: serde_json::Value, out: &mut dyn io::Write) -> io::Result<()> {
    write(Rust::from(json), out)
}

struct Rust {
    root: String,
    structs: Vec<Struct>,
    enums: Vec<Enum>,
}

struct Struct {
    name: String,
    fields: Vec<StructField>,
}

struct Enum {
    name: String,
    variants: Vec<EnumVariant>,
}

struct StructField {
    original_name: String,
    var_name: String,
    type_name: String,
}

struct EnumVariant {
    variant_name: String,
    associated_type: String,
}

impl From<serde_json::Value> for Rust {
    fn from(json: serde_json::Value) -> Self {
        let type_graph = TypeGraph::from(json);
        let name_registry = NameRegistry::build(
            &type_graph,
            NamePreference {
                filter: |name: &str| is_rust_identifier(name),
                compare: |a: &str, b: &str| a.cmp(b),
            },
        );
        let back_edges = back_edges(&type_graph);

        let mut root = String::from("serde_json::Value");
        let mut structs = vec![];
        let mut enums = vec![];

        if let Some(type_def) = type_graph.type_def(type_graph.root) {
            match type_def {
                TypeDef::Object(_) => {
                    root = derive_type_name(
                        type_graph.root,
                        &type_graph,
                        &name_registry,
                        type_graph.root,
                        &back_edges,
                    )
                }
                TypeDef::Array(inner_type_id) => {
                    root = format!(
                        "Vec<{}>",
                        derive_type_name(
                            *inner_type_id,
                            &type_graph,
                            &name_registry,
                            type_graph.root,
                            &back_edges
                        )
                    )
                }
                _ => {
                    root = derive_type_name(
                        type_graph.root,
                        &type_graph,
                        &name_registry,
                        type_graph.root,
                        &back_edges,
                    )
                }
            };
        }

        for (type_id, type_def) in &type_graph {
            if let TypeDef::Object(object_fields) = type_def {
                let struct_name = name_registry
                    .assigned_name(type_id)
                    .map(|ident| ident.to_case(Case::Pascal))
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut struct_fields: Vec<StructField> = Vec::with_capacity(object_fields.len());
                for (idx, object_field) in object_fields.iter().enumerate() {
                    let original_name = object_field.name.clone();
                    let type_name = derive_type_name(
                        object_field.type_id,
                        &type_graph,
                        &name_registry,
                        type_id,
                        &back_edges,
                    );
                    let var_name = match is_rust_identifier(&object_field.name) {
                        true => object_field.name.to_case(Case::Snake),
                        false => format!("var_{}", idx),
                    };

                    struct_fields.push(StructField {
                        original_name,
                        type_name,
                        var_name,
                    });
                }

                structs.push(Struct {
                    name: struct_name,
                    fields: struct_fields,
                });
            }

            if let TypeDef::Union(inner_type_ids) = type_def {
                let enum_name = name_registry
                    .assigned_name(type_id)
                    .map(|ident| ident.to_case(Case::Pascal))
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut variants: Vec<EnumVariant> = Vec::with_capacity(inner_type_ids.len());
                for inner_type_id in inner_type_ids {
                    let variant_type = derive_type_name(
                        *inner_type_id,
                        &type_graph,
                        &name_registry,
                        type_id,
                        &back_edges,
                    );
                    let variant_name = match type_graph.type_def(*inner_type_id) {
                        Some(inner_type_def) => match inner_type_def {
                            TypeDef::String => "String".into(),
                            TypeDef::Integer => "Int".into(),
                            TypeDef::Float => "Float".into(),
                            TypeDef::Boolean => "Bool".into(),
                            TypeDef::Null => "Null".into(),
                            TypeDef::Unknown => "Unknown".into(),
                            TypeDef::Object(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Snake))
                                .unwrap_or_else(|| format!("Object{}", inner_type_id)),
                            TypeDef::Union(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Snake))
                                .unwrap_or_else(|| format!("Union{}", inner_type_id)),
                            TypeDef::Array(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Snake))
                                .unwrap_or_else(|| format!("Array{}", inner_type_id)),
                            TypeDef::Optional(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Snake))
                                .unwrap_or_else(|| format!("Optional{}", inner_type_id)),
                        },
                        None => format!("Variant{}", inner_type_id),
                    };

                    variants.push(EnumVariant {
                        variant_name,
                        associated_type: variant_type,
                    });
                }

                enums.push(Enum {
                    name: enum_name,
                    variants,
                });
            }
        }

        Self {
            root,
            structs,
            enums,
        }
    }
}

fn back_edges(type_graph: &TypeGraph) -> Vec<(TypeId, TypeId)> {
    let mut back_edges = vec![];

    let mut path: Vec<TypeId> = vec![];
    let mut frontier: Vec<TypeId> = vec![type_graph.root];

    while let Some(type_id) = frontier.pop() {
        path.push(type_id);
        if let Some(type_def) = type_graph.type_def(type_id) {
            let adj_type_ids: Box<dyn Iterator<Item = usize>> = match type_def {
                TypeDef::Object(object_fields) => Box::new(object_fields.iter().map(|f| f.type_id)),
                TypeDef::Union(inner_type_ids) => Box::new(inner_type_ids.into_iter().copied()),
                TypeDef::Array(inner_type_id) | TypeDef::Optional(inner_type_id) => {
                    Box::new(iter::once(*inner_type_id))
                }
                _ => Box::new(iter::empty()),
            };

            for adj_type_id in adj_type_ids {
                match path.contains(&adj_type_id) {
                    true => {
                        back_edges.push((type_id, adj_type_id));
                        path.pop();
                    }
                    false => frontier.push(adj_type_id),
                }
            }
        }
    }

    back_edges
}

fn derive_type_name(
    type_id: TypeId,
    type_graph: &TypeGraph,
    name_registry: &NameRegistry,
    parent_type_id: TypeId,
    back_edges: &[(TypeId, TypeId)],
) -> String {
    match type_graph.type_def(type_id) {
        Some(type_def) => match type_def {
            TypeDef::String => "String".into(),
            TypeDef::Integer => "isize".into(),
            TypeDef::Float => "f64".into(),
            TypeDef::Boolean => "bool".into(),
            TypeDef::Null | TypeDef::Unknown => "Option<serde_json::Value>".into(),
            TypeDef::Object(_) | TypeDef::Union(_) => {
                let mut ident = name_registry
                    .assigned_name(type_id)
                    .map(|ident| ident.to_case(Case::Pascal))
                    .unwrap_or_else(|| format!("Type{}", type_id));
                if back_edges.contains(&(parent_type_id, type_id)) {
                    ident = format!("Box<{}>", ident);
                }
                ident
            }
            TypeDef::Array(inner_type_id) => format!(
                "Vec<{}>",
                derive_type_name(
                    *inner_type_id,
                    type_graph,
                    name_registry,
                    type_id,
                    back_edges
                )
            ),
            TypeDef::Optional(inner_type_id) => {
                let mut inner_type_name = derive_type_name(
                    *inner_type_id,
                    type_graph,
                    name_registry,
                    type_id,
                    back_edges,
                );
                if back_edges.contains(&(parent_type_id, type_id)) {
                    inner_type_name = format!("Box<{}>", inner_type_name);
                }
                format!("Option<{}>", inner_type_name)
            }
        },
        None => format!("Unknown{}", type_id),
    }
}

fn is_rust_identifier(s: &str) -> bool {
    syn::parse_str::<syn::Ident>(s).is_ok()
}

fn write(rust: Rust, out: &mut dyn io::Write) -> io::Result<()> {
    writeln!(out, "use serde::{{Serialize, Deserialize}};")?;

    // struct with name ROOT (SCREAMING_SNAKE_CASE)
    // will never clash with other structs (PascalCase)
    writeln!(out, "// entry point = ROOT")?;
    writeln!(out, "pub type ROOT = {};", rust.root)?;

    for def in rust.structs {
        writeln!(out, "#[derive(Serialize, Deserialize, Debug)]")?;
        writeln!(out, "pub struct {} {{", def.name)?;
        for field in def.fields {
            if field.original_name != field.var_name {
                writeln!(out, "    #[serde(rename = \"{}\")]", field.original_name)?;
            }
            writeln!(out, "    pub {}: {},", field.var_name, field.type_name)?;
        }
        writeln!(out, "}}")?;
    }

    for def in rust.enums {
        writeln!(out, "#[derive(Serialize, Deserialize, Debug)]")?;
        writeln!(out, "#[serde(untagged)]")?;
        writeln!(out, "pub enum {} {{", def.name)?;
        for variant in def.variants {
            writeln!(
                out,
                "    {}({}),",
                variant.variant_name, variant.associated_type
            )?;
        }
        writeln!(out, "}}")?;
    }

    Ok(())
}
