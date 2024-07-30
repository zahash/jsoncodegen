use serde_json::{Map, Value};

#[derive(Debug)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Unknown,
    Object(String),
    Array(Box<FieldType>),
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub type_: FieldType,
}

#[derive(Debug)]
pub struct Structure {
    pub name: String,
    pub fields: Vec<Field>,
}

pub fn process(json: Value) -> Vec<Structure> {
    let mut structs = vec![];

    match json {
        Value::Array(arr) => {
            process_array(arr, "Root".into(), &mut structs);
        }
        Value::Object(obj) => {
            process_object(obj, "Root".into(), &mut structs);
        }
        _ => unreachable!("Valid top level Value will always be object or array"),
    }

    structs
}

fn process_array(arr: Vec<Value>, name: String, structs: &mut Vec<Structure>) -> FieldType {
    match arr.into_iter().next() {
        Some(value) => determine_type(value, name, structs),
        None => FieldType::Unknown,
    }
}

fn process_object(obj: Map<String, Value>, name: String, structs: &mut Vec<Structure>) {
    let mut fields = vec![];

    for (key, value) in obj {
        let type_ = determine_type(value, key.clone(), structs);
        fields.push(Field { name: key, type_ })
    }

    structs.push(Structure {
        name: name.clone(),
        fields,
    });
}

fn determine_type(value: Value, name: String, structs: &mut Vec<Structure>) -> FieldType {
    match value {
        Value::Null => FieldType::Unknown,
        Value::Bool(_) => FieldType::Boolean,
        Value::Number(n) => match n.is_f64() {
            true => FieldType::Float,
            false => FieldType::Integer,
        },
        Value::String(_) => FieldType::String,
        Value::Array(arr) => FieldType::Array(Box::new(process_array(arr, name, structs))),
        Value::Object(nested_obj) => {
            process_object(nested_obj, name.clone(), structs);
            FieldType::Object(name.clone())
        }
    }
}
