use super::{to_camel_case_or_unknown, to_pascal_case_or_unknown, Iota};
use crate::schema::{Field, FieldType, Schema};
use std::io::{Error, Write};

pub fn java<W: Write>(schema: Schema, out: &mut W) -> Result<(), Error> {
    let mut ctx = Context::new();

    match schema {
        Schema::Object(fields) => ctx.add_class("Root".into(), fields),
        Schema::Array(ty) => {
            ctx.process_field(Field {
                name: "Item".into(),
                ty,
            });
        }
    };

    for class in ctx.classes {
        writeln!(out, "// {}.java", class.name)?;
        writeln!(out, "import com.fasterxml.jackson.annotation.*;")?;

        writeln!(out, "public class {} {{", class.name)?;
        for member_var in &class.vars {
            writeln!(
                out,
                "    private {} {};",
                member_var.type_name, member_var.var_name
            )?;
        }

        for member_var in &class.vars {
            let add_json_property = member_var.original_name != member_var.var_name;
            if add_json_property {
                writeln!(out, "    @JsonProperty(\"{}\")", member_var.original_name)?;
            }
            writeln!(
                out,
                "    public {} get{}() {{ return {}; }}",
                member_var.type_name,
                to_pascal_case_or_unknown(&member_var.var_name, &mut ctx.iota),
                member_var.var_name
            )?;
            if add_json_property {
                writeln!(out, "    @JsonProperty(\"{}\")", member_var.original_name)?;
            }
            writeln!(
                out,
                "    public void set{}({} value) {{ this.{} = value; }}",
                to_pascal_case_or_unknown(&member_var.var_name, &mut ctx.iota),
                member_var.type_name,
                member_var.var_name
            )?;
        }

        writeln!(out, "}}")?;
    }

    for union in ctx.unions {
        writeln!(out, "// {}.java", union.name)?;
        writeln!(out, "import java.io.IOException;")?;
        writeln!(out, "import com.fasterxml.jackson.core.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.*;")?;
        writeln!(out, "import com.fasterxml.jackson.databind.annotation.*;")?;

        writeln!(
            out,
            "@JsonSerialize(using = {}.Serializer.class)",
            union.name
        )?;
        writeln!(
            out,
            "@JsonDeserialize(using = {}.Deserializer.class)",
            union.name
        )?;
        writeln!(out, "public class {} {{", union.name)?;

        for union_var in &union.vars {
            writeln!(
                out,
                "    public {} {};",
                union_var.type_name, union_var.var_name
            )?;
        }

        // Serializer
        writeln!(
            out,
            "    static class Serializer extends JsonSerializer<{}> {{",
            union.name
        )?;
        writeln!(out, "        @Override public void serialize({} value, JsonGenerator generator, SerializerProvider serializer) throws IOException {{", union.name)?;
        for union_var in &union.vars {
            writeln!(
                out,
                "            if (value.{} != null) {{ generator.writeObject(value.{}); return; }}",
                union_var.var_name, union_var.var_name
            )?;
        }
        writeln!(out, "            generator.writeNull();")?;
        writeln!(out, "        }}")?;
        writeln!(out, "    }}")?;

        // Deserializer
        writeln!(
            out,
            "    static class Deserializer extends JsonDeserializer<{}> {{",
            union.name
        )?;
        writeln!(out, "        @Override public {} deserialize(JsonParser parser, DeserializationContext ctx) throws IOException {{", union.name)?;
        writeln!(
            out,
            "            {} value = new {}();",
            union.name, union.name
        )?;
        writeln!(out, "            switch (parser.currentToken()) {{")?;

        writeln!(out, "            case VALUE_NULL: break;")?;
        for union_var in &union.vars {
            match union_var.type_name.as_str() {
                "String" => writeln!(out, "            case VALUE_STRING: value.{} = parser.readValueAs(String.class); break;", union_var.var_name)?,
                "Long" => writeln!(out, "            case VALUE_NUMBER_INT: value.{} = parser.readValueAs(Long.class); break;", union_var.var_name)?,
                "Double" => writeln!(out, "            case VALUE_NUMBER_FLOAT: value.{} = parser.readValueAs(Double.class); break;", union_var.var_name)?,
                "Boolean" => writeln!(out, "            case VALUE_TRUE: case VALUE_FALSE: value.{} = parser.readValueAs(Boolean.class); break;", union_var.var_name)?,
                _ if union_var.type_name.starts_with("List") => writeln!(out, "            case START_ARRAY: value.{} = parser.readValueAs({}.class); break;", union_var.var_name, union_var.type_name)?,
                _ => writeln!(out, "            case START_OBJECT: value.{} = parser.readValueAs({}.class); break;", union_var.var_name, union_var.type_name)?,
            };
        }
        writeln!(
            out,
            "            default: throw new IOException(\"Cannot deserialize {}\");",
            union.name
        )?;
        writeln!(out, "            }}")?;
        writeln!(out, "            return value;")?;
        writeln!(out, "        }}")?;
        writeln!(out, "    }}")?;
        writeln!(out, "}}")?;
    }

    Ok(())
}

struct Context {
    classes: Vec<Class>,
    unions: Vec<Union>,
    iota: Iota,
}

struct Class {
    name: String,
    vars: Vec<MemberVar>,
}

struct MemberVar {
    original_name: String,
    var_name: String,
    type_name: String,
}

struct Union {
    name: String,
    vars: Vec<UnionMemberVar>,
}

struct UnionMemberVar {
    var_name: String,
    type_name: String,
}

impl Context {
    fn new() -> Self {
        Self {
            classes: vec![],
            unions: vec![],
            iota: Iota::new(),
        }
    }

    fn add_class(&mut self, name: String, fields: Vec<Field>) {
        let mut class = Class {
            name: name.clone(),
            vars: vec![],
        };

        for field in fields {
            class.vars.push(self.process_field(field));
        }

        self.classes.push(class);
    }

    fn add_union_class(&mut self, name: String, variants: Vec<FieldType>) {
        let mut union = Union {
            name: name.clone(),
            vars: vec![],
        };

        for variant in variants {
            union
                .vars
                .push(self.process_union_field(name.clone(), variant));
        }

        self.unions.push(union);
    }

    fn process_field(&mut self, field: Field) -> MemberVar {
        match field.ty {
            FieldType::String => MemberVar {
                var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "String".into(),
            },
            FieldType::Integer => MemberVar {
                var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "Long".into(),
            },
            FieldType::Float => MemberVar {
                var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "Double".into(),
            },
            FieldType::Boolean => MemberVar {
                var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "Boolean".into(),
            },
            FieldType::Unknown => MemberVar {
                var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "Object".into(),
            },
            FieldType::Object(nested_fields) => {
                let nested_class_name = to_pascal_case_or_unknown(&field.name, &mut self.iota);
                self.add_class(nested_class_name.clone(), nested_fields);
                MemberVar {
                    var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                    original_name: field.name,
                    type_name: nested_class_name,
                }
            }
            FieldType::Union(types) => {
                let nested_class_name = to_pascal_case_or_unknown(&field.name, &mut self.iota);
                self.add_union_class(nested_class_name.clone(), types);
                MemberVar {
                    var_name: to_camel_case_or_unknown(&field.name, &mut self.iota),
                    original_name: field.name,
                    type_name: nested_class_name,
                }
            }
            FieldType::Array(ty) => {
                let mut member_var = self.process_field(Field {
                    name: field.name,
                    ty: *ty,
                });
                member_var.type_name = format!("List<{}>", member_var.type_name);
                member_var
            }
            FieldType::Optional(ty) => self.process_field(Field {
                name: field.name,
                ty: *ty,
            }),
        }
    }

    fn process_union_field(&mut self, prefix: String, variant: FieldType) -> UnionMemberVar {
        match variant {
            FieldType::String => UnionMemberVar {
                var_name: "strVal".into(),
                type_name: "String".into(),
            },
            FieldType::Integer => UnionMemberVar {
                var_name: "longVal".into(),
                type_name: "Long".into(),
            },
            FieldType::Float => UnionMemberVar {
                var_name: "doubleVal".into(),
                type_name: "Double".into(),
            },
            FieldType::Boolean => UnionMemberVar {
                var_name: "boolVal".into(),
                type_name: "Boolean".into(),
            },
            FieldType::Unknown => UnionMemberVar {
                var_name: "objVal".into(),
                type_name: "Object".into(),
            },
            FieldType::Object(fields) => {
                let member_var = self.process_field(Field {
                    name: prefix + "Clazz",
                    ty: FieldType::Object(fields),
                });

                UnionMemberVar {
                    var_name: member_var.var_name,
                    type_name: member_var.type_name,
                }
            }
            FieldType::Union(types) => {
                let member_var = self.process_field(Field {
                    name: prefix + "Ele",
                    ty: FieldType::Union(types),
                });

                UnionMemberVar {
                    var_name: member_var.var_name,
                    type_name: member_var.type_name,
                }
            }
            FieldType::Array(ty) => {
                let member_var = self.process_field(Field {
                    name: prefix + "Arr",
                    ty: FieldType::Array(ty),
                });

                UnionMemberVar {
                    var_name: member_var.var_name,
                    type_name: member_var.type_name,
                }
            }
            FieldType::Optional(ty) => {
                let member_var = self.process_field(Field {
                    name: prefix + "Opt",
                    ty: FieldType::Optional(ty),
                });

                UnionMemberVar {
                    var_name: member_var.var_name,
                    type_name: member_var.type_name,
                }
            }
        }
    }
}
