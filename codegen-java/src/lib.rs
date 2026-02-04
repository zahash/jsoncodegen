use std::collections::{HashMap, HashSet, BTreeMap};
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
    root: RootType,
    classes: Vec<Class>,
    unions: Vec<Union>,
}

enum RootType {
    Extension(String), // extends ...
    Wrapper(String),   // wrapper around ...
}

struct Class {
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
    name: String,
    vars: Vec<UnionMemberVar>,
}

#[derive(Clone)]
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

        let java_names = build_java_names(&type_graph, &name_registry);

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
                        &java_names,
                    ))
                }
                TypeDef::Array(inner_type_id) => {
                    root = RootType::Extension(format!(
                        "java.util.ArrayList<{}>",
                        derive_type_name(*inner_type_id, &type_graph, &java_names)
                    ))
                }
                _ => {
                    root = RootType::Wrapper(derive_type_name(
                        type_graph.root,
                        &type_graph,
                        &java_names,
                    ))
                }
            };
        }

        // Use a sorted list of TypeIds for deterministic output
        let mut type_ids: Vec<&TypeId> = type_graph.nodes.keys().collect();
        type_ids.sort();

        for type_id in type_ids {
            let type_def = type_graph.nodes.get(type_id).unwrap();

            if let TypeDef::Object(object_fields) = type_def {
                let class_name = java_names
                    .get(type_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut vars: Vec<MemberVar> = Vec::with_capacity(object_fields.len());
                let mut used_var_names = HashSet::new();
                let mut needs_custom_serializer_deserializer = false;

                for (idx, object_field) in object_fields.iter().enumerate() {
                    let original_name = object_field.name.clone();
                    if original_name.is_empty() {
                        needs_custom_serializer_deserializer = true;
                    }
                    let type_name =
                        derive_type_name(object_field.type_id, &type_graph, &java_names);

                    let camel_name = object_field.name.to_case(Case::Camel);
                    let base_var_name = if is_java_identifier(&camel_name) {
                        camel_name
                    } else {
                        format!("var{}", idx)
                    };

                    // Dedup variable names
                    let mut var_name = base_var_name.clone();
                    let mut count = 2;
                    while used_var_names.contains(&var_name) {
                        var_name = format!("{}{}", base_var_name, count);
                        count += 1;
                    }
                    used_var_names.insert(var_name.clone());

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
                    name: class_name,
                    vars,
                    needs_custom_serializer_deserializer,
                });
            }

            if let TypeDef::Union(inner_type_ids) = type_def {
                let class_name = java_names
                    .get(type_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut vars: Vec<UnionMemberVar> = Vec::with_capacity(inner_type_ids.len());
                let mut used_var_names = HashSet::new();

                for inner_type_id in inner_type_ids {
                    let type_name = derive_type_name(*inner_type_id, &type_graph, &java_names);
                    let raw_var_name = match type_graph.nodes.get(inner_type_id) {
                        Some(inner_type_def) => match inner_type_def {
                            TypeDef::String => "strVal".into(),
                            TypeDef::Integer => "intVal".into(),
                            TypeDef::Float => "doubleVal".into(),
                            TypeDef::Boolean => "boolVal".into(),
                            TypeDef::Null => "nullVal".into(),
                            TypeDef::Unknown => "objVal".into(),
                            TypeDef::Object(_) => java_names
                                .get(inner_type_id)
                                .map(|name| name.to_case(Case::Camel))
                                .unwrap_or_else(|| format!("clazz{}", inner_type_id)),
                            TypeDef::Union(_) => java_names
                                .get(inner_type_id)
                                .map(|name| name.to_case(Case::Camel))
                                .unwrap_or_else(|| format!("union{}", inner_type_id)),
                            TypeDef::Array(_) => format!("arr{}", inner_type_id),
                            TypeDef::Optional(_) => format!("opt{}", inner_type_id),
                        },
                        None => format!("variant{}", inner_type_id),
                    };

                    let base_var_name = if is_java_identifier(&raw_var_name) {
                        raw_var_name
                    } else {
                        format!("variant{}", inner_type_id)
                    };

                    // Dedup union variable names
                    let mut var_name = base_var_name.clone();
                    let mut count = 2;
                    while used_var_names.contains(&var_name) {
                        var_name = format!("{}{}", base_var_name, count);
                        count += 1;
                    }
                    used_var_names.insert(var_name.clone());

                    vars.push(UnionMemberVar {
                        var_name,
                        type_name,
                    });
                }

                unions.push(Union {
                    name: class_name,
                    vars,
                });
            }
        }

        Self {
            root,
            classes,
            unions,
        }
    }
}

fn build_java_names(type_graph: &TypeGraph, name_registry: &NameRegistry) -> HashMap<TypeId, String> {
    let mut map = HashMap::new();
    let mut used_names = HashSet::new();

    let mut type_ids: Vec<&TypeId> = type_graph.nodes.keys().collect();
    type_ids.sort();

    for type_id in type_ids {
        let type_def = type_graph.nodes.get(type_id).unwrap();
        match type_def {
            TypeDef::Object(_) | TypeDef::Union(_) => {
                let base_name = name_registry
                    .assigned_name(*type_id)
                    .map(|ident| ident.to_case(Case::Pascal))
                    .unwrap_or_else(|| format!("Type{}", type_id));

                let mut name = base_name.clone();
                let mut count = 2;
                // Ensure name is unique
                while used_names.contains(&name) {
                    name = format!("{}{}", base_name, count);
                    count += 1;
                }

                used_names.insert(name.clone());
                map.insert(*type_id, name);
            }
            _ => {}
        }
    }
    map
}

fn derive_type_name(
    type_id: TypeId,
    type_graph: &TypeGraph,
    java_names: &HashMap<TypeId, String>,
) -> String {
    match type_graph.nodes.get(&type_id) {
        Some(type_def) => match type_def {
            TypeDef::String => "String".into(),
            TypeDef::Integer => "Long".into(),
            TypeDef::Float => "Double".into(),
            TypeDef::Boolean => "Boolean".into(),
            TypeDef::Null | TypeDef::Unknown => "Object".into(),
            TypeDef::Object(_) | TypeDef::Union(_) => java_names
                .get(&type_id)
                .cloned()
                .unwrap_or_else(|| format!("Type{}", type_id)),
            TypeDef::Array(inner_type_id) => format!(
                "{}[]",
                derive_type_name(*inner_type_id, type_graph, java_names)
            ),
            TypeDef::Optional(inner_type_id) => {
                derive_type_name(*inner_type_id, type_graph, java_names)
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

fn write(java: Java, out: &mut dyn io::Write) -> io::Result<()> {
    if java
        .classes
        .iter()
        .flat_map(|c| &c.vars)
        .any(|v| v.annotate)
        || matches!(java.root, RootType::Wrapper(_))
    {
        writeln!(out, "import com.fasterxml.jackson.annotation.*;")?;
    }

    if !java.unions.is_empty()
        || java
            .classes
            .iter()
            .any(|c| c.needs_custom_serializer_deserializer)
    {
        writeln!(out, "import com.fasterxml.jackson.core.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.annotation.*;")?;
        writeln!(out, "import java.io.IOException;")?;
    }

    writeln!(out, "public class JsonCodeGen {{")?;

    // class with name ROOT (SCREAMING_SNAKE_CASE)
    // will never clash with other classes (PascalCase)
    writeln!(out, "\t// entry point = ROOT")?;
    match java.root {
        RootType::Extension(base) => {
            writeln!(out, "\tpublic static class ROOT extends {} {{}}", base)?;
        }
        RootType::Wrapper(inner) => {
            writeln!(out, "\tpublic static class ROOT {{")?;
            writeln!(out, "\t\tprivate final {} value;", inner)?;
            writeln!(out, "\t\t@JsonCreator(mode = JsonCreator.Mode.DELEGATING)")?;
            writeln!(out, "\t\tpublic ROOT({} value) {{ this.value = value; }}", inner)?;
            writeln!(out, "\t\t@JsonValue")?;
            writeln!(out, "\t\tpublic {} getValue() {{ return value; }}", inner)?;
            writeln!(out, "\t}}")?;
        }
    }

    for class in java.classes {
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
        for member_var in &class.vars {
            writeln!(
                out,
                "\t\tprivate {} {};",
                member_var.type_name, member_var.var_name
            )?;
        }

        for member_var in &class.vars {
            if member_var.annotate {
                writeln!(out, "\t\t@JsonProperty({:?})", member_var.original_name)?;
            }
            writeln!(
                out,
                "\t\tpublic {} {}() {{ return {}; }}",
                member_var.type_name, member_var.getter_name, member_var.var_name
            )?;
            if member_var.annotate {
                writeln!(out, "\t\t@JsonProperty({:?})", member_var.original_name)?;
            }
            writeln!(
                out,
                "\t\tpublic void {}({} value) {{ this.{} = value; }}",
                member_var.setter_name, member_var.type_name, member_var.var_name
            )?;
        }

        if class.needs_custom_serializer_deserializer {
            // --- Custom Serializer ---
            writeln!(
                out,
                "\t\tstatic class Serializer extends JsonSerializer<{}> {{",
                class.name
            )?;
            writeln!(
                out,
                "\t\t\t@Override public void serialize({} value, JsonGenerator gen, SerializerProvider serializers) throws IOException {{",
                class.name
            )?;
            writeln!(out, "\t\t\t\tgen.writeStartObject();")?;

            for var in &class.vars {
                writeln!(out, "\t\t\t\tif (value.{} != null) {{", var.var_name)?;
                writeln!(
                    out,
                    "\t\t\t\t\tgen.writeFieldName({:?});",
                    var.original_name
                )?;
                writeln!(
                    out,
                    "\t\t\t\t\tserializers.defaultSerializeValue(value.{}, gen);",
                    var.var_name
                )?;
                writeln!(out, "\t\t\t\t}}")?;
            }

            writeln!(out, "\t\t\t\tgen.writeEndObject();")?;
            writeln!(out, "\t\t\t}}")?;
            writeln!(out, "\t\t}}")?;

            // --- Custom Deserializer ---
            writeln!(
                out,
                "\t\tstatic class Deserializer extends JsonDeserializer<{}> {{",
                class.name
            )?;
            writeln!(
                out,
                "\t\t\t@Override public {} deserialize(JsonParser p, DeserializationContext ctxt) throws IOException {{",
                class.name
            )?;
            writeln!(
                out,
                "\t\t\t\t{} instance = new {}();",
                class.name, class.name
            )?;
            writeln!(
                out,
                "\t\t\t\twhile (p.nextToken() != JsonToken.END_OBJECT) {{"
            )?;
            writeln!(out, "\t\t\t\t\tString fieldName = p.currentName();")?;
            writeln!(out, "\t\t\t\t\tp.nextToken();")?;

            writeln!(out, "\t\t\t\t\tif (fieldName == null) {{ continue; }}")?;

            for (i, var) in class.vars.iter().enumerate() {
                let check = if i == 0 { "if" } else { "else if" };
                writeln!(
                    out,
                    "\t\t\t\t\t{} ({:?}.equals(fieldName)) {{",
                    check, var.original_name
                )?;
                writeln!(
                    out,
                    "\t\t\t\t\t\tinstance.{} = ctxt.readValue(p, {}.class);",
                    var.var_name, var.type_name
                )?;
                writeln!(out, "\t\t\t\t\t}}")?;
            }

            writeln!(out, "\t\t\t\t\telse {{ p.skipChildren(); }}")?;
            writeln!(out, "\t\t\t\t}}")?;
            writeln!(out, "\t\t\t\treturn instance;")?;
            writeln!(out, "\t\t\t}}")?;
            writeln!(out, "\t\t}}")?;
        }

        writeln!(out, "\t}}")?;
    }

    for union in java.unions {
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

        for union_var in &union.vars {
            writeln!(
                out,
                "\t\tpublic {} {};",
                union_var.type_name, union_var.var_name
            )?;
        }

        // Serializer
        writeln!(
            out,
            "\t\tstatic class Serializer extends JsonSerializer<{}> {{",
            union.name
        )?;
        writeln!(
            out,
            "\t\t\t@Override public void serialize({} value, JsonGenerator generator, SerializerProvider serializer) throws IOException {{",
            union.name
        )?;
        for union_var in &union.vars {
            writeln!(
                out,
                "\t\t\t\tif (value.{} != null) {{ generator.writeObject(value.{}); return; }}",
                union_var.var_name, union_var.var_name
            )?;
        }
        writeln!(out, "\t\t\t\tgenerator.writeNull();")?;
        writeln!(out, "\t\t\t}}")?;
        writeln!(out, "\t\t}}")?;

        // Deserializer
        writeln!(
            out,
            "\t\tstatic class Deserializer extends JsonDeserializer<{}> {{",
            union.name
        )?;
        writeln!(
            out,
            "\t\t\t@Override public {} deserialize(JsonParser parser, DeserializationContext ctx) throws IOException {{",
            union.name
        )?;
        writeln!(out, "\t\t\t\t{} value = new {}();", union.name, union.name)?;
        writeln!(out, "\t\t\t\tswitch (parser.currentToken()) {{")?;

        // Group variants by their expected JsonToken
        let mut variants_by_token: BTreeMap<String, Vec<&UnionMemberVar>> = BTreeMap::new();

        for union_var in &union.vars {
            let token = match union_var.type_name.as_str() {
                "String" => "VALUE_STRING",
                "Long" => "VALUE_NUMBER_INT",
                "Double" => "VALUE_NUMBER_FLOAT",
                "Boolean" => "VALUE_TRUE", // Special handling needed for TRUE/FALSE
                _ if union_var.type_name.ends_with("[]") => "START_ARRAY",
                _ => "START_OBJECT",
            };

            if token == "VALUE_TRUE" {
                variants_by_token.entry("VALUE_TRUE".to_string()).or_default().push(union_var);
                variants_by_token.entry("VALUE_FALSE".to_string()).or_default().push(union_var);
            } else {
                variants_by_token.entry(token.to_string()).or_default().push(union_var);
            }
        }

        writeln!(out, "\t\t\t\tcase VALUE_NULL: break;")?;

        for (token, vars) in variants_by_token {
            writeln!(out, "\t\t\t\tcase {}:", token)?;

            if vars.len() > 1 && (token == "START_OBJECT" || token == "START_ARRAY") {
                // Ambiguous union: deserialize as tree and try mapping to each variant
                writeln!(out, "\t\t\t\t\tJsonNode tree = parser.readValueAsTree();")?;
                writeln!(out, "\t\t\t\t\tObjectMapper mapper = (ObjectMapper) parser.getCodec();")?;

                for union_var in vars {
                    writeln!(out, "\t\t\t\t\ttry {{")?;
                    writeln!(out, "\t\t\t\t\t\tvalue.{} = mapper.treeToValue(tree, {}.class);", union_var.var_name, union_var.type_name)?;
                    writeln!(out, "\t\t\t\t\t\tbreak;")?;
                    writeln!(out, "\t\t\t\t\t}} catch (Exception e) {{}}")?;
                }
            } else {
                if let Some(first_var) = vars.first() {
                    let read_expr = match first_var.type_name.as_str() {
                        "String" => "parser.readValueAs(String.class)",
                        "Long" => "parser.readValueAs(Long.class)",
                        "Double" => "parser.readValueAs(Double.class)",
                        "Boolean" => "parser.readValueAs(Boolean.class)",
                        _ => &format!("parser.readValueAs({}.class)", first_var.type_name),
                    };
                    writeln!(out, "\t\t\t\t\tvalue.{} = {};", first_var.var_name, read_expr)?;
                }
            }
            writeln!(out, "\t\t\t\t\tbreak;")?;
        }

        writeln!(
            out,
            "\t\t\t\tdefault: throw new IOException(\"Cannot deserialize {}\");",
            union.name
        )?;
        writeln!(out, "\t\t\t\t}}")?;
        writeln!(out, "\t\t\t\treturn value;")?;
        writeln!(out, "\t\t\t}}")?;
        writeln!(out, "\t\t}}")?;
        writeln!(out, "\t}}")?;
    }

    writeln!(out, "}}")
}
