
use crate::{
    flat_schema::{FlatField, FlatFieldType, FlatObject, FlatUnion},
    iota::Iota,
};

pub struct TypeTable {
    objects: Vec<FlatObject>,
    unions: Vec<FlatUnion>,
    iota: Iota,
}

impl TypeTable {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            unions: Vec::new(),
            iota: Iota::new(),
        }
    }

    pub fn types(self) -> (Vec<FlatObject>, Vec<FlatUnion>) {
        (self.objects, self.unions)
    }

    pub fn register_object(&mut self, mut fields: Vec<FlatField>) -> usize {
        fields.sort_by(|a, b| a.name.cmp(&b.name));

        for obj in &self.objects {
            if obj.fields == fields {
                return obj.id;
            }
        }

        let id = self.iota.next();
        self.objects.push(FlatObject { id, fields });
        id
    }

    pub fn register_union(&mut self, mut field_types: Vec<FlatFieldType>) -> usize {
        field_types.sort();

        for un in &self.unions {
            if un.field_types == field_types {
                return un.id;
            }
        }

        let id = self.iota.next();
        self.unions.push(FlatUnion { id, field_types });
        id
    }
}
