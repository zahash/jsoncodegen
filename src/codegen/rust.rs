use super::CaseConverter;
use crate::schema::{Field, FieldType, Schema};
use std::io::{Error, Write};

pub fn rust<W: Write>(schema: Schema, out: &mut W) -> Result<(), Error> {
    let mut ctx = Context::new();
    writeln!(out, "use serde::{{Serialize, Deserialize}};")?;

    match schema {
        Schema::Object(fields) => ctx.add_struct("Root".into(), fields),
        Schema::Array(ty) => {
            ctx.process_field(Field {
                name: "Item".into(),
                ty,
            });
            ctx.add_alias("Root".into(), "Vec<Item>".into());
        }
    };

    for def in ctx.aliases {
        writeln!(out, "type {} = {};", def.name, def.ty)?;
    }

    for def in ctx.structs {
        writeln!(out, "#[derive(Serialize, Deserialize, Debug)]")?;
        writeln!(out, "pub struct {} {{", def.name)?;
        for field in def.fields {
            if field.original_name != field.variable_name {
                writeln!(out, "    #[serde(rename = \"{}\")]", field.original_name)?;
            }
            writeln!(out, "    pub {}: {},", field.variable_name, field.type_name)?;
        }
        writeln!(out, "}}")?;
    }

    for def in ctx.enums {
        writeln!(out, "#[derive(Serialize, Deserialize, Debug)]")?;
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

struct Context {
    aliases: Vec<AliasDef>,
    structs: Vec<StructDef>,
    enums: Vec<EnumDef>,
    case_converter: CaseConverter,
}

struct StructDef {
    name: String,
    fields: Vec<StructField>,
}

struct EnumDef {
    name: String,
    variants: Vec<EnumVariant>,
}

struct AliasDef {
    name: String,
    ty: String,
}

struct StructField {
    original_name: String,
    variable_name: String,
    type_name: String,
}

struct EnumVariant {
    variant_name: String,
    associated_type: String,
}

impl Context {
    fn new() -> Self {
        Self {
            aliases: vec![],
            structs: vec![],
            enums: vec![],
            case_converter: CaseConverter::new(),
        }
    }

    fn add_alias(&mut self, name: String, ty: String) {
        self.aliases.push(AliasDef { name, ty });
    }

    fn add_struct(&mut self, name: String, fields: Vec<Field>) {
        let mut def = StructDef {
            name,
            fields: vec![],
        };

        for field in fields {
            def.fields.push(self.process_field(field));
        }

        // TODO
        // struct field_name might have duplicates.
        // eg: "123foo" and "fooあ" will both resolve to "foo"

        self.structs.push(def);
    }

    fn add_enum(&mut self, name: String, variants: Vec<FieldType>) {
        let mut def = EnumDef {
            name: name.clone(),
            variants: vec![],
        };

        for variant in variants {
            def.variants
                .push(self.process_enum_variant(name.clone(), variant));
        }

        self.enums.push(def);
    }

    fn process_field(&mut self, field: Field) -> StructField {
        match field.ty {
            FieldType::String => StructField {
                variable_name: self.case_converter.snake_case(&field.name),
                original_name: field.name,
                type_name: "String".into(),
            },
            FieldType::Integer => StructField {
                variable_name: self.case_converter.snake_case(&field.name),
                original_name: field.name,
                type_name: "isize".into(),
            },
            FieldType::Float => StructField {
                variable_name: self.case_converter.snake_case(&field.name),
                original_name: field.name,
                type_name: "f64".into(),
            },
            FieldType::Boolean => StructField {
                variable_name: self.case_converter.snake_case(&field.name),
                original_name: field.name,
                type_name: "bool".into(),
            },
            FieldType::Unknown => StructField {
                variable_name: self.case_converter.snake_case(&field.name),
                original_name: field.name,
                type_name: "serde_json::Value".into(),
            },
            FieldType::Object(nested_fields) => {
                let nested_struct_name = self.case_converter.pascal_case(&field.name);
                self.add_struct(nested_struct_name.clone(), nested_fields);
                StructField {
                    variable_name: self.case_converter.snake_case(&field.name),
                    original_name: field.name,
                    type_name: nested_struct_name,
                }
            }
            FieldType::Union(types) => {
                let nested_enum_name = self.case_converter.pascal_case(&field.name);
                self.add_enum(nested_enum_name.clone(), types);
                StructField {
                    variable_name: self.case_converter.snake_case(&field.name),
                    original_name: field.name,
                    type_name: nested_enum_name,
                }
            }
            FieldType::Array(ty) => {
                let mut struct_field = self.process_field(Field {
                    name: field.name,
                    ty: *ty,
                });
                struct_field.type_name = format!("Vec<{}>", struct_field.type_name);
                struct_field
            }
            FieldType::Optional(ty) => {
                let mut struct_field = self.process_field(Field {
                    name: field.name,
                    ty: *ty,
                });
                struct_field.type_name = format!("Option<{}>", struct_field.type_name);
                struct_field
            }
        }
    }

    fn process_enum_variant(&mut self, prefix: String, variant: FieldType) -> EnumVariant {
        match variant {
            FieldType::String => EnumVariant {
                variant_name: "String".into(),
                associated_type: "String".into(),
            },
            FieldType::Integer => EnumVariant {
                variant_name: "Integer".into(),
                associated_type: "isize".into(),
            },
            FieldType::Float => EnumVariant {
                variant_name: "Float".into(),
                associated_type: "f64".into(),
            },
            FieldType::Boolean => EnumVariant {
                variant_name: "Boolean".into(),
                associated_type: "bool".into(),
            },
            FieldType::Unknown => EnumVariant {
                variant_name: "Unknown".into(),
                associated_type: "serde_json::Value".into(),
            },
            FieldType::Object(fields) => {
                let struct_field = self.process_field(Field {
                    name: prefix + "Class",
                    ty: FieldType::Object(fields),
                });

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
            FieldType::Union(types) => {
                let struct_field = self.process_field(Field {
                    name: prefix + "Element",
                    ty: FieldType::Union(types),
                });

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
            FieldType::Array(ty) => {
                let struct_field = self.process_field(Field {
                    name: prefix + "Array",
                    ty: FieldType::Array(ty),
                });

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
            FieldType::Optional(ty) => {
                let struct_field = self.process_field(Field {
                    name: prefix + "Optional",
                    ty: FieldType::Optional(ty),
                });

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
        }
    }
}
