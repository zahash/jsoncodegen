use super::{
    case::{to_pascal_case_or_unknown, to_snake_case_or_unknown},
    Iota,
};
use crate::schema::{Field, FieldType, Schema};
use std::io::{Error, Write};

pub fn rust<W: Write>(schema: Schema, out: &mut W) -> Result<(), Error> {
    let mut ctx = Context::new();
    writeln!(out, "use serde::{{Serialize, Deserialize}};")?;

    match schema {
        Schema::Object(fields) => {
            ctx.add_struct("Root".into(), fields);
        }
        Schema::Array(ty) => {
            let struct_field = ctx.process_field(
                "Root".into(),
                Field {
                    name: "Item".into(),
                    ty,
                },
            );
            ctx.add_alias("Root".into(), format!("Vec<{}>", struct_field.type_name));
        }
    };

    for def in ctx.aliases {
        writeln!(out, "pub type {} = {};", def.name, def.ty)?;
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
    iota: Iota,
}

#[derive(PartialEq)]
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

#[derive(PartialEq)]
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
            iota: Iota::new(),
        }
    }

    fn add_alias(&mut self, name: String, ty: String) {
        self.aliases.push(AliasDef { name, ty });
    }

    fn add_struct(&mut self, name: String, fields: Vec<Field>) -> String {
        // let mut def = StructDef {
        //     name,
        //     fields: vec![],
        // };

        let mut struct_def_fields = vec![];

        for field in fields {
            struct_def_fields.push(self.process_field(name.clone(), field));
        }

        // TODO
        // struct field_name might have duplicates.
        // eg: "123foo" and "fooあ" will both resolve to "foo"

        /*
        TODO: this should've been
        THIS IS DIFFICULT TODO AND NON-CRITICAL!!
        struct Root {
            val: isize,
            next: Option<Box<Root>>,
        }
        {
            "val": 10,
            "next": {
                "val": 20,
                "next": {
                    "val": 30,
                    "next": 10
                }
            }
        }

        TODO: different structs might with same names.
        to avoid this the process_field must also take the parent name as argument
        name of nested struct must be combination of parent name and field name
        {
            "val": 10,
            "next": {
                "val": 20,
                "next": {
                    "val": 30,
                    "next": null
                }
            }
        }


        {
            "from": { "x": 0, "y": 0 },
            "to": { "x": 1, "y": 1 },
            "nest": {
                "from": { "a": "b", "c": "d" }
            }
        }


         */

        // self.structs.push(StructDef { name: name.clone(), fields: struct_def_fields });
        // name

        match self
            .structs
            .iter()
            .find(|StructDef { name: _, fields }| *fields == struct_def_fields)
        {
            Some(StructDef { name, fields: _ }) => name.clone(),
            None => {
                // TODO: check if there is a different struct with the same name
                // nested structs might have same name but different fields

                self.structs.push(StructDef {
                    name: name.clone(),
                    fields: struct_def_fields,
                });
                name
            }
        }
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

    fn process_field(&mut self, parent_name: String, field: Field) -> StructField {
        match field.ty {
            FieldType::String => StructField {
                variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "String".into(),
            },
            FieldType::Integer => StructField {
                variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "isize".into(),
            },
            FieldType::Float => StructField {
                variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "f64".into(),
            },
            FieldType::Boolean => StructField {
                variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "bool".into(),
            },
            FieldType::Unknown => StructField {
                variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                original_name: field.name,
                type_name: "serde_json::Value".into(),
            },
            FieldType::Object(nested_fields) => {
                let nested_struct_name =
                    to_pascal_case_or_unknown(&(parent_name + " " + &field.name), &mut self.iota);
                let nested_struct_name = self.add_struct(nested_struct_name, nested_fields);
                StructField {
                    variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                    original_name: field.name,
                    type_name: nested_struct_name,
                }
            }
            FieldType::Union(types) => {
                let nested_enum_name =
                    to_pascal_case_or_unknown(&(parent_name + " " + &field.name), &mut self.iota);
                self.add_enum(nested_enum_name.clone(), types);
                StructField {
                    variable_name: to_snake_case_or_unknown(&field.name, &mut self.iota),
                    original_name: field.name,
                    type_name: nested_enum_name,
                }
            }
            FieldType::Array(ty) => {
                let mut struct_field = self.process_field(
                    parent_name,
                    Field {
                        name: field.name,
                        ty: *ty,
                    },
                );
                struct_field.type_name = format!("Vec<{}>", struct_field.type_name);
                struct_field
            }
            FieldType::Optional(ty) => {
                let mut struct_field = self.process_field(
                    parent_name,
                    Field {
                        name: field.name,
                        ty: *ty,
                    },
                );
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
                let struct_field = self.process_field(
                    prefix.clone() + "Class",
                    Field {
                        name: prefix + "Class",
                        ty: FieldType::Object(fields),
                    },
                );

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
            FieldType::Union(types) => {
                let struct_field = self.process_field(
                    prefix.clone() + "Element",
                    Field {
                        name: prefix + "Element",
                        ty: FieldType::Union(types),
                    },
                );

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
            FieldType::Array(ty) => {
                let struct_field = self.process_field(
                    prefix.clone() + "Array",
                    Field {
                        name: prefix + "Array",
                        ty: FieldType::Array(ty),
                    },
                );

                EnumVariant {
                    variant_name: to_pascal_case_or_unknown(
                        &struct_field.variable_name,
                        &mut self.iota,
                    ),
                    associated_type: struct_field.type_name,
                }
            }
            FieldType::Optional(ty) => {
                let struct_field = self.process_field(
                    prefix.clone() + "Optional",
                    Field {
                        name: prefix + "Optional",
                        ty: FieldType::Optional(ty),
                    },
                );

                EnumVariant {
                    variant_name: struct_field.type_name.clone(),
                    associated_type: struct_field.type_name,
                }
            }
        }
    }
}
