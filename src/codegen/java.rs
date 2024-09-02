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

pub fn java<W: Write>(schema: Structure, opts: &JavaOpts, out: &mut W) -> Result<(), Error> {
    writeln!(
        out,
        "{} class {} {{",
        opts.class_access_modifier.to_string(),
        schema.name
    )?;

    for field in schema.fields {
        let java_type = match field.type_ {
            FieldType::String => "String".into(),
            FieldType::Integer => "Long".into(),
            FieldType::Float => "Double".into(),
            FieldType::Boolean => "Boolean".into(),
            FieldType::Unknown => "Object".into(),
            FieldType::Object(obj) => {
                let obj_name = obj.name.clone();
                java(obj, &opts, out)?;
                obj_name
            }
            FieldType::Array(types) => {
                format!("List<>")
            }
        };
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

        if opts.getters {
            writeln!(
                out,
                "    @JsonProperty(\"{}\") public {} get{}() {{ return {}; }}",
                &field.name, java_type, &field.name, &field.name
            )?;
        }

        if opts.setters {
            writeln!(
                out,
                "    @JsonProperty(\"{}\") public void set{}({} {}) {{ this.{} = {}; }}",
                &field.name, &field.name, java_type, &field.name, &field.name, &field.name
            )?;
        }
    }

    writeln!(out, "}}")?;

    Ok(())
}

fn field_type_to_java_type(field_type: FieldType) -> String {
    match field_type {
        FieldType::String => "String".into(),
        FieldType::Integer => "Long".into(),
        FieldType::Float => "Double".into(),
        FieldType::Boolean => "Boolean".into(),
        FieldType::Unknown => "Object".into(),
        FieldType::Object(obj) => name.into(),
        FieldType::Array(types) => format!("List<{}>", unify_field_types(types)),
    }
}

fn unify_field_types(field_types: &[FieldType]) -> String {
    todo!()
}
