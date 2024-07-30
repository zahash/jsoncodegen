use crate::schema_extraction::{FieldType, Structure};
use std::io::{Error, Write};

pub fn java<W: Write>(schema: &[Structure], out: &mut W) -> Result<(), Error> {
    for class in schema {
        writeln!(out, "class {} {{", class.name)?;

        for field in &class.fields {
            let java_type = field_type_to_java_type(&field.type_);
            writeln!(out, "    {} {};", java_type, field.name)?;
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
