use crate::schema_extraction::{FieldType, Structure};
use clap::{Parser, ValueEnum};
use std::io::{Error, Write};

#[derive(Parser, Debug)]
pub struct JavaOpts {
    #[arg(short, long, default_value_t = JavaAccessModifier::Public)]
    class_access_modifier: JavaAccessModifier,

    #[arg(short, long, default_value_t = JavaAccessModifier::Public)]
    attribute_access_modifier: JavaAccessModifier,

    #[arg(short, long)]
    final_attributes: bool,

    #[arg(short, long)]
    getters: bool,

    #[arg(short, long)]
    setters: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum JavaAccessModifier {
    Public,
    Private,
    Protected,
    Default,
}

impl ToString for JavaAccessModifier {
    fn to_string(&self) -> String {
        match self {
            JavaAccessModifier::Public => "public",
            JavaAccessModifier::Private => "private",
            JavaAccessModifier::Protected => "protected",
            JavaAccessModifier::Default => "",
        }
        .into()
    }
}

pub fn java<W: Write>(schema: &[Structure], opts: JavaOpts, out: &mut W) -> Result<(), Error> {
    for class in schema {
        writeln!(
            out,
            "{} class {} {{",
            opts.class_access_modifier.to_string(),
            class.name
        )?;

        for field in &class.fields {
            let java_type = field_type_to_java_type(&field.type_);
            writeln!(
                out,
                "    {} {} {} {};",
                opts.attribute_access_modifier.to_string(),
                match opts.final_attributes {
                    true => "final",
                    false => "",
                },
                java_type,
                field.name
            )?;
        }

        if opts.getters {
            for field in &class.fields {
                let java_type = field_type_to_java_type(&field.type_);
                writeln!(
                    out,
                    "    public {} get{}() {{ return {}; }}",
                    java_type, &field.name, &field.name
                )?;
            }
        }

        if opts.setters {
            for field in &class.fields {
                let java_type = field_type_to_java_type(&field.type_);
                writeln!(
                    out,
                    "    public void set{}({} {}) {{ this.{} = {}; }}",
                    &field.name, java_type, &field.name, &field.name, &field.name
                )?;
            }
        }

        writeln!(out, "}}")?;
    }

    Ok(())
}

fn field_type_to_java_type(field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => "String".into(),
        FieldType::Integer => "Integer".into(),
        FieldType::Float => "Double".into(),
        FieldType::Boolean => "Boolean".into(),
        FieldType::Unknown => "Object".into(),
        FieldType::Object(name) => name.into(),
        FieldType::Array(type_) => format!("List<{}>", field_type_to_java_type(type_)),
    }
}
