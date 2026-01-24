use std::io;

use convert_case::{Case, Casing};
use jsoncodegen::{
    name_registry::{NamePreference, NameRegistry},
    type_graph::{TypeDef, TypeGraph, TypeId},
};
use unicode_general_category::{GeneralCategory, get_general_category};

pub fn codegen(json: serde_json::Value, out: &mut dyn io::Write) -> io::Result<()> {
    write(Java::from(json), out)
}

struct Java {
    root_id: TypeId,
    root: RootType,
    classes: Vec<Class>,
    unions: Vec<Union>,
}

enum RootType {
    Extension(String), // extends ...
    Wrapper(String),   // wrapper around ...
}

struct Class {
    type_id: TypeId,
    name: String,
    vars: Vec<MemberVar>,
    needs_custom_serializer_deserializer: bool,
}

struct MemberVar {
    original_name: String,
    type_name: String,
    var_name: String,
    getter_name: String,
    setter_name: String,
    annotate: bool,
}

struct Union {
    type_id: TypeId,
    name: String,
    vars: Vec<UnionMemberVar>,
}

struct UnionMemberVar {
    var_name: String,
    type_name: String,
}

impl From<serde_json::Value> for Java {
    fn from(json: serde_json::Value) -> Self {
        let type_graph = TypeGraph::from(json);
        let name_registry = NameRegistry::build(
            &type_graph,
            NamePreference {
                filter: |name: &str| is_java_identifier(name),
                compare: |a: &str, b: &str| a.cmp(b),
            },
        );

        let mut root = RootType::Extension("Object".into());
        let mut classes = vec![];
        let mut unions = vec![];

        // Determine root type
        if let Some(type_def) = type_graph.nodes.get(&type_graph.root) {
            match type_def {
                TypeDef::Object(_) => {
                    root = RootType::Extension(derive_type_name(
                        type_graph.root,
                        &type_graph,
                        &name_registry,
                    ))
                }
                TypeDef::Array(inner_type_id) => {
                    root = RootType::Extension(format!(
                        "java.util.ArrayList<{}>",
                        derive_type_name(*inner_type_id, &type_graph, &name_registry)
                    ))
                }
                _ => {
                    let type_name = match type_def {
                        TypeDef::String => "String".into(),
                        TypeDef::Integer => "Long".into(),
                        TypeDef::Float => "Double".into(),
                        TypeDef::Boolean => "Boolean".into(),
                        TypeDef::Null | TypeDef::Unknown => "Object".into(),
                        TypeDef::Optional(inner) => {
                            derive_type_name(*inner, &type_graph, &name_registry)
                        }
                        TypeDef::Array(inner) => format!(
                            "{}[]",
                            derive_type_name(*inner, &type_graph, &name_registry)
                        ),
                        TypeDef::Object(_) | TypeDef::Union(_) => "JsonCodeGen".into(),
                    };
                    root = RootType::Wrapper(type_name)
                }
            };
        }

        for (type_id, type_def) in &type_graph.nodes {
            if let TypeDef::Object(object_fields) = type_def {
                let class_name = name_registry
                    .assigned_name(*type_id)
                    .map(|ident| ident.to_case(Case::Pascal))
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut vars: Vec<MemberVar> = Vec::with_capacity(object_fields.len());
                let mut needs_custom_serializer_deserializer = false;
                for (idx, object_field) in object_fields.iter().enumerate() {
                    let original_name = object_field.name.clone();
                    if original_name.is_empty() {
                        needs_custom_serializer_deserializer = true;
                    }
                    let type_name =
                        derive_type_name(object_field.type_id, &type_graph, &name_registry);
                    let var_name = match is_java_identifier(&object_field.name) {
                        true => object_field.name.to_case(Case::Camel),
                        false => format!("var{}", idx),
                    };
                    let getter_name = format!("get{}", var_name.to_case(Case::Pascal));
                    let setter_name = format!("set{}", var_name.to_case(Case::Pascal));
                    let annotate =
                        decapitalize_java(&var_name.to_case(Case::Pascal)) != original_name;

                    vars.push(MemberVar {
                        original_name,
                        type_name,
                        var_name,
                        getter_name,
                        setter_name,
                        annotate,
                    });
                }

                // don't need to annotate if custom serializer/deserializer is used
                if needs_custom_serializer_deserializer {
                    for var in &mut vars {
                        var.annotate = false;
                    }
                }

                classes.push(Class {
                    type_id: *type_id,
                    name: class_name,
                    vars,
                    needs_custom_serializer_deserializer,
                });
            }

            if let TypeDef::Union(inner_type_ids) = type_def {
                let class_name = name_registry
                    .assigned_name(*type_id)
                    .map(|ident| ident.to_case(Case::Pascal))
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut vars: Vec<UnionMemberVar> = Vec::with_capacity(inner_type_ids.len());
                for inner_type_id in inner_type_ids {
                    let type_name = derive_type_name(*inner_type_id, &type_graph, &name_registry);
                    let var_name = match type_graph.nodes.get(inner_type_id) {
                        Some(inner_type_def) => match inner_type_def {
                            TypeDef::String => "strVal".into(),
                            TypeDef::Integer => "intVal".into(),
                            TypeDef::Float => "doubleVal".into(),
                            TypeDef::Boolean => "boolVal".into(),
                            TypeDef::Null => "nullVal".into(),
                            TypeDef::Unknown => "objVal".into(),
                            TypeDef::Object(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Camel))
                                .unwrap_or_else(|| format!("clazz{}", inner_type_id)),
                            TypeDef::Union(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Camel))
                                .unwrap_or_else(|| format!("union{}", inner_type_id)),
                            TypeDef::Array(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Camel))
                                .unwrap_or_else(|| format!("arr{}", inner_type_id)),
                            TypeDef::Optional(_) => name_registry
                                .assigned_name(*inner_type_id)
                                .map(|ident| ident.to_case(Case::Camel))
                                .unwrap_or_else(|| format!("opt{}", inner_type_id)),
                        },
                        None => format!("variant{}", inner_type_id),
                    };

                    vars.push(UnionMemberVar {
                        var_name,
                        type_name,
                    });
                }

                unions.push(Union {
                    type_id: *type_id,
                    name: class_name,
                    vars,
                });
            }
        }

        Self {
            root_id: type_graph.root,
            root,
            classes,
            unions,
        }
    }
}

fn derive_type_name(
    type_id: TypeId,
    type_graph: &TypeGraph,
    name_registry: &NameRegistry,
) -> String {
    if type_id == type_graph.root {
        return "JsonCodeGen".into();
    }
    match type_graph.nodes.get(&type_id) {
        Some(type_def) => match type_def {
            TypeDef::String => "String".into(),
            TypeDef::Integer => "Long".into(),
            TypeDef::Float => "Double".into(),
            TypeDef::Boolean => "Boolean".into(),
            TypeDef::Null | TypeDef::Unknown => "Object".into(),
            TypeDef::Object(_) | TypeDef::Union(_) => name_registry
                .assigned_name(type_id)
                .map(|ident| ident.to_case(Case::Pascal))
                .unwrap_or_else(|| format!("Type{}", type_id)),
            TypeDef::Array(inner_type_id) => format!(
                "{}[]",
                derive_type_name(*inner_type_id, type_graph, name_registry)
            ),
            TypeDef::Optional(inner_type_id) => {
                derive_type_name(*inner_type_id, type_graph, name_registry)
            }
        },
        None => format!("Unknown{}", type_id),
    }
}

fn is_java_identifier(s: &str) -> bool {
    if is_java_keyword_or_literal(s) {
        return false;
    }

    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    is_java_identifier_start(first) && chars.all(is_java_identifier_part)
}

fn is_java_keyword_or_literal(s: &str) -> bool {
    // https://docs.oracle.com/javase/tutorial/java/nutsandbolts/_keywords.html
    match s {
        "_" | // Java 9+ single underscore is a keyword
        "true" | "false" | "null" | // literals
        // Keywords (JLS 3.9)
        "abstract" | "assert" | "boolean" | "break" | "byte" | "case" | "catch" | "char"
        | "class" | "const" | "continue" | "default" | "do" | "double" | "else" | "enum"
        | "extends" | "final" | "finally" | "float" | "for" | "goto" | "if"
        | "implements" | "import" | "instanceof" | "int" | "interface" | "long" | "native"
        | "new"  | "package" | "private" | "protected" | "public" | "return" | "short"
        | "static" | "strictfp" | "super" | "switch" | "synchronized" | "this" | "throw"
        | "throws" | "transient" | "try" | "void" | "volatile" | "while" => true,
        _ => false,
    }
}

fn is_java_identifier_start(ch: char) -> bool {
    matches!(
        get_general_category(ch),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::LetterNumber
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ConnectorPunctuation
    )
}

fn is_java_identifier_part(ch: char) -> bool {
    is_java_identifier_start(ch)
        || matches!(
            get_general_category(ch),
            GeneralCategory::DecimalNumber
                | GeneralCategory::SpacingMark
                | GeneralCategory::NonspacingMark
                | GeneralCategory::Format
        )
}

/// Java Beans decapitalize rule
pub fn decapitalize_java(s: &str) -> String {
    let mut chars = s.chars();

    let Some(first) = chars.next() else {
        return String::new();
    };

    match chars.next() {
        Some(second) if first.is_uppercase() && second.is_uppercase() => s.to_string(),
        Some(second) => {
            let mut out = first.to_lowercase().collect::<String>();
            out.push(second);
            out.extend(chars);
            out
        }
        None => first.to_lowercase().collect::<String>(),
    }
}

fn write_class_body(
    class: &Class,
    class_name: &str,
    indent: &str,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    for member_var in &class.vars {
        writeln!(
            out,
            "{}private {} {};",
            indent, member_var.type_name, member_var.var_name
        )?;
    }

    for member_var in &class.vars {
        if member_var.annotate {
            writeln!(out, "{}@JsonProperty({:?})", indent, member_var.original_name)?;
        }
        writeln!(
            out,
            "{}public {} {}() {{ return {}; }}",
            indent, member_var.type_name, member_var.getter_name, member_var.var_name
        )?;
        if member_var.annotate {
            writeln!(out, "{}@JsonProperty({:?})", indent, member_var.original_name)?;
        }
        writeln!(
            out,
            "{}public void {}({} value) {{ this.{} = value; }}",
            indent, member_var.setter_name, member_var.type_name, member_var.var_name
        )?;
    }

    if class.needs_custom_serializer_deserializer {
        // --- Custom Serializer ---
        writeln!(
            out,
            "{}static class Serializer extends JsonSerializer<{}> {{",
            indent, class_name
        )?;
        writeln!(
            out,
            "{}\t@Override public void serialize({} value, JsonGenerator gen, SerializerProvider serializers) throws IOException {{",
            indent, class_name
        )?;
        writeln!(out, "{}\t\tgen.writeStartObject();", indent)?;

        for var in &class.vars {
            writeln!(out, "{}\t\tif (value.{} != null) {{", indent, var.var_name)?;
            writeln!(
                out,
                "{}\t\t\tgen.writeFieldName({:?});",
                indent, var.original_name
            )?;
            writeln!(
                out,
                "{}\t\t\tserializers.defaultSerializeValue(value.{}, gen);",
                indent, var.var_name
            )?;
            writeln!(out, "{}\t\t}}", indent)?;
        }

        writeln!(out, "{}\t\tgen.writeEndObject();", indent)?;
        writeln!(out, "{}\t}}", indent)?;
        writeln!(out, "{}}}", indent)?;

        // --- Custom Deserializer ---
        writeln!(
            out,
            "{}static class Deserializer extends JsonDeserializer<{}> {{",
            indent, class_name
        )?;
        writeln!(
            out,
            "{}\t@Override public {} deserialize(JsonParser p, DeserializationContext ctxt) throws IOException {{",
            indent, class_name
        )?;
        writeln!(
            out,
            "{}\t\t{} instance = new {}();",
            indent, class_name, class_name
        )?;
        writeln!(
            out,
            "{}\t\twhile (p.nextToken() != JsonToken.END_OBJECT) {{",
            indent
        )?;
        writeln!(out, "{}\t\t\tString fieldName = p.currentName();", indent)?;
        writeln!(out, "{}\t\t\tp.nextToken();", indent)?;

        writeln!(out, "{}\t\t\tif (fieldName == null) {{ continue; }}", indent)?;

        for (i, var) in class.vars.iter().enumerate() {
            let check = if i == 0 { "if" } else { "else if" };
            writeln!(
                out,
                "{}\t\t\t{} ({:?}.equals(fieldName)) {{",
                indent, check, var.original_name
            )?;
            writeln!(
                out,
                "{}\t\t\t\tinstance.{} = ctxt.readValue(p, {}.class);",
                indent, var.var_name, var.type_name
            )?;
            writeln!(out, "{}\t\t\t}}", indent)?;
        }

        writeln!(out, "{}\t\t\telse {{ p.skipChildren(); }}", indent)?;
        writeln!(out, "{}\t\t}}", indent)?;
        writeln!(out, "{}\t\treturn instance;", indent)?;
        writeln!(out, "{}\t}}", indent)?;
        writeln!(out, "{}}}", indent)?;
    }
    Ok(())
}

fn write_union_body(
    union: &Union,
    union_name: &str,
    indent: &str,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    for union_var in &union.vars {
        writeln!(
            out,
            "{}public {} {};",
            indent, union_var.type_name, union_var.var_name
        )?;
    }

    // Serializer
    writeln!(
        out,
        "{}static class Serializer extends JsonSerializer<{}> {{",
        indent, union_name
    )?;
    writeln!(
        out,
        "{}\t@Override public void serialize({} value, JsonGenerator generator, SerializerProvider serializer) throws IOException {{",
        indent, union_name
    )?;
    for union_var in &union.vars {
        writeln!(
            out,
            "{}\t\tif (value.{} != null) {{ generator.writeObject(value.{}); return; }}",
            indent, union_var.var_name, union_var.var_name
        )?;
    }
    writeln!(out, "{}\t\tgenerator.writeNull();", indent)?;
    writeln!(out, "{}\t}}", indent)?;
    writeln!(out, "{}}}", indent)?;

    // Deserializer
    writeln!(
        out,
        "{}static class Deserializer extends JsonDeserializer<{}> {{",
        indent, union_name
    )?;
    writeln!(
        out,
        "{}\t@Override public {} deserialize(JsonParser parser, DeserializationContext ctx) throws IOException {{",
        indent, union_name
    )?;
    writeln!(
        out,
        "{}\t\t{} value = new {}();",
        indent, union_name, union_name
    )?;
    writeln!(
        out,
        "{}\t\tswitch (parser.currentToken()) {{",
        indent
    )?;

    writeln!(out, "{}\t\tcase VALUE_NULL: break;", indent)?;
    for union_var in &union.vars {
        match union_var.type_name.as_str() {
            "String" => writeln!(
                out,
                "{}\t\tcase VALUE_STRING: value.{} = parser.readValueAs(String.class); break;",
                indent, union_var.var_name
            )?,
            "Long" => writeln!(
                out,
                "{}\t\tcase VALUE_NUMBER_INT: value.{} = parser.readValueAs(Long.class); break;",
                indent, union_var.var_name
            )?,
            "Double" => writeln!(
                out,
                "{}\t\tcase VALUE_NUMBER_FLOAT: value.{} = parser.readValueAs(Double.class); break;",
                indent, union_var.var_name
            )?,
            "Boolean" => writeln!(
                out,
                "{}\t\tcase VALUE_TRUE: case VALUE_FALSE: value.{} = parser.readValueAs(Boolean.class); break;",
                indent, union_var.var_name
            )?,
            _ if union_var.type_name.ends_with("[]") => writeln!(
                out,
                "{}\t\tcase START_ARRAY: value.{} = parser.readValueAs({}.class); break;",
                indent, union_var.var_name, union_var.type_name
            )?,
            _ => writeln!(
                out,
                "{}\t\tcase START_OBJECT: value.{} = parser.readValueAs({}.class); break;",
                indent, union_var.var_name, union_var.type_name
            )?,
        };
    }
    writeln!(
        out,
        "{}\t\tdefault: throw new IOException(\"Cannot deserialize {}\");",
        indent, union_name
    )?;
    writeln!(out, "{}\t\t}}", indent)?;
    writeln!(out, "{}\t\treturn value;", indent)?;
    writeln!(out, "{}\t}}", indent)?;
    writeln!(out, "{}}}", indent)?;
    Ok(())
}

fn write(java: Java, out: &mut dyn io::Write) -> io::Result<()> {
    let root_class = java.classes.iter().find(|c| c.type_id == java.root_id);
    let root_union = java.unions.iter().find(|u| u.type_id == java.root_id);

    let mut needs_annotations = java
        .classes
        .iter()
        .flat_map(|c| &c.vars)
        .any(|v| v.annotate)
        || matches!(java.root, RootType::Wrapper(_));

    if let Some(c) = root_class {
        if c.vars.iter().any(|v| v.annotate) {
            needs_annotations = true;
        }
    }

    if needs_annotations {
        writeln!(out, "import com.fasterxml.jackson.annotation.*;")?;
    }

    let mut needs_jackson = !java.unions.is_empty()
        || java
            .classes
            .iter()
            .any(|c| c.needs_custom_serializer_deserializer);

    if let Some(c) = root_class {
        if c.needs_custom_serializer_deserializer {
            needs_jackson = true;
        }
    }
    if root_union.is_some() {
        needs_jackson = true;
    }

    if needs_jackson {
        writeln!(out, "import com.fasterxml.jackson.core.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.annotation.*;")?;
        writeln!(out, "import java.io.IOException;")?;
    }

    if let Some(c) = root_class {
        if c.needs_custom_serializer_deserializer {
            writeln!(out, "@JsonSerialize(using = JsonCodeGen.Serializer.class)")?;
            writeln!(
                out,
                "@JsonDeserialize(using = JsonCodeGen.Deserializer.class)"
            )?;
        }
    }
    if root_union.is_some() {
        writeln!(out, "@JsonSerialize(using = JsonCodeGen.Serializer.class)")?;
        writeln!(
            out,
            "@JsonDeserialize(using = JsonCodeGen.Deserializer.class)"
        )?;
    }

    // Determine generation strategy based on root type
    if root_class.is_some() || root_union.is_some() {
        // Root is a class or union, so we inline it into JsonCodeGen
        writeln!(out, "public class JsonCodeGen {{")?;
    } else {
        // Root is likely Array or Primitive, use legacy logic
        match java.root {
            RootType::Extension(base) => {
                writeln!(out, "public class JsonCodeGen extends {} {{", base)?;
            }
            RootType::Wrapper(inner) => {
                writeln!(out, "public class JsonCodeGen {{")?;
                writeln!(out, "\tprivate final {} value;", inner)?;
                writeln!(out, "\t@JsonCreator(mode = JsonCreator.Mode.DELEGATING)")?;
                writeln!(
                    out,
                    "\tpublic JsonCodeGen({} value) {{ this.value = value; }}",
                    inner
                )?;
                writeln!(out, "\t@JsonValue")?;
                writeln!(out, "\tpublic {} getValue() {{ return value; }}", inner)?;
            }
        }
    }

    if let Some(class) = root_class {
        write_class_body(class, "JsonCodeGen", "\t", out)?;
    } else if let Some(union) = root_union {
        write_union_body(union, "JsonCodeGen", "\t", out)?;
    }

    for class in java.classes {
        if class.type_id == java.root_id {
            continue;
        }

        if class.needs_custom_serializer_deserializer {
            writeln!(
                out,
                "\t@JsonSerialize(using = {}.Serializer.class)",
                class.name
            )?;
            writeln!(
                out,
                "\t@JsonDeserialize(using = {}.Deserializer.class)",
                class.name
            )?;
        }
        writeln!(out, "\tpublic static class {} {{", class.name)?;
        write_class_body(&class, &class.name, "\t\t", out)?;
        writeln!(out, "\t}}")?;
    }

    for union in java.unions {
        if union.type_id == java.root_id {
            continue;
        }
        writeln!(
            out,
            "\t@JsonSerialize(using = {}.Serializer.class)",
            union.name
        )?;
        writeln!(
            out,
            "\t@JsonDeserialize(using = {}.Deserializer.class)",
            union.name
        )?;
        writeln!(out, "\tpublic static class {} {{", union.name)?;
        write_union_body(&union, &union.name, "\t\t", out)?;
        writeln!(out, "\t}}")?;
    }

    writeln!(out, "}}")
}

#[cfg(test)]
