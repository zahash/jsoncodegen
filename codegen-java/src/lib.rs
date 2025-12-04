use std::io;

use jsoncodegen::{name_registry::NameRegistry, type_graph::TypeGraph};

pub fn codegen(json: serde_json::Value, out: &mut dyn io::Write) -> io::Result<()> {
    write(Java::from(json), out)
}

struct Java {
    classes: Vec<Class>,
    unions: Vec<Union>,
}

struct Class {
    name: String,
    vars: Vec<MemberVar>,
}

struct MemberVar {
    original_name: String,
    type_name: String,
    var_name: String,
    getter_name: String,
    setter_name: String,
}

struct Union {
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
        let name_registry = NameRegistry::build(&type_graph);

        todo!()
    }
}

fn write(java: Java, out: &mut dyn io::Write) -> io::Result<()> {
    if !java.classes.is_empty() {
        writeln!(out, "import com.fasterxml.jackson.annotation.*;")?;
    }

    if !java.unions.is_empty() {
        writeln!(out, "import java.io.IOException;")?;
        writeln!(out, "import com.fasterxml.jackson.core.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.annotation.*;")?;
    }

    writeln!(out, "public class JsonCodeGen {{")?;

    for class in java.classes {
        writeln!(out, "\tpublic static class {} {{", class.name)?;
        for member_var in &class.vars {
            writeln!(
                out,
                "\t\tprivate {} {};",
                member_var.type_name, member_var.var_name
            )?;
        }

        for member_var in &class.vars {
            let add_json_property = member_var.original_name != member_var.var_name;
            if add_json_property {
                writeln!(out, "\t\t@JsonProperty({:?})", member_var.original_name)?;
            }
            writeln!(
                out,
                "\t\tpublic {} {}() {{ return {}; }}",
                member_var.type_name, member_var.getter_name, member_var.var_name
            )?;
            if add_json_property {
                writeln!(out, "\t\t@JsonProperty({:?})", member_var.original_name)?;
            }
            writeln!(
                out,
                "\t\tpublic void {}({} value) {{ this.{} = value; }}",
                member_var.setter_name, member_var.type_name, member_var.var_name
            )?;
        }

        writeln!(out, "}}")?;
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

        writeln!(out, "\t\t\t\tcase VALUE_NULL: break;")?;
        for union_var in &union.vars {
            match union_var.type_name.as_str() {
                "String" => writeln!(
                    out,
                    "\t\t\t\tcase VALUE_STRING: value.{} = parser.readValueAs(String.class); break;",
                    union_var.var_name
                )?,
                "Long" => writeln!(
                    out,
                    "\t\t\t\tcase VALUE_NUMBER_INT: value.{} = parser.readValueAs(Long.class); break;",
                    union_var.var_name
                )?,
                "Double" => writeln!(
                    out,
                    "\t\t\t\tcase VALUE_NUMBER_FLOAT: value.{} = parser.readValueAs(Double.class); break;",
                    union_var.var_name
                )?,
                "Boolean" => writeln!(
                    out,
                    "\t\t\t\tcase VALUE_TRUE: case VALUE_FALSE: value.{} = parser.readValueAs(Boolean.class); break;",
                    union_var.var_name
                )?,
                _ if union_var.type_name.starts_with("List") => writeln!(
                    out,
                    "\t\t\t\tcase START_ARRAY: value.{} = parser.readValueAs({}.class); break;",
                    union_var.var_name, union_var.type_name
                )?,
                _ => writeln!(
                    out,
                    "\t\t\t\tcase START_OBJECT: value.{} = parser.readValueAs({}.class); break;",
                    union_var.var_name, union_var.type_name
                )?,
            };
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
