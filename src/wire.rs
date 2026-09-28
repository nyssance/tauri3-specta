//! Normalize standard-library metadata to the JSON representation used by Tauri.
use std::collections::BTreeMap;

use specta::{
    Type, Types,
    datatype::{DataType, Enum, Field, Fields, NamedReferenceType, Reference, Variant},
};

// A named recursive type avoids expanding serde_json::Value indefinitely.
// JSON numbers become JavaScript numbers; large integers have JS precision limits.
#[allow(dead_code)]
#[derive(Type, serde::Serialize)]
#[serde(untagged)]
enum JsonValue {
    Null(()),
    Bool(bool),
    Number(#[specta(type = specta_typescript::Number)] f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

pub(crate) struct WireTypes {
    original: Types,
    json: Option<DataType>,
    number: DataType,
}

impl WireTypes {
    pub(crate) fn new(types: &mut Types) -> Self {
        let original = types.clone();
        let has_json = original
            .into_unsorted_iter()
            .any(|ty| ty.module_path == "serde_json" && ty.name == "Value");
        let json = has_json.then(|| JsonValue::definition(types));
        let wire = Self {
            original,
            json,
            number: specta_typescript::Number::<()>::definition(types),
        };
        types.iter_mut(|ty| {
            if ty.module_path == "serde_json" && ty.name == "Value" {
                ty.ty = Some(wire.json.clone().expect("JSON type was registered"));
            } else if ty.module_path == "serde_json" && ty.name == "Number" {
                ty.ty = Some(wire.number.clone());
            } else if ["std::result", "core::result"].contains(&ty.module_path.as_ref())
                && ty.name == "Result"
            {
                normalize_result(ty.ty.as_mut().expect("Result definition"));
            } else if let Some(body) = &mut ty.ty {
                wire.normalize(body);
            }
        });
        wire
    }

    pub(crate) fn normalize(&self, datatype: &mut DataType) {
        if let DataType::Reference(Reference::Named(reference)) = datatype
            && let Some(ty) = self.original.get(reference)
        {
            if ty.module_path == "serde_json" && ty.name == "Value" {
                *datatype = self.json.clone().expect("JSON type was registered");
                return;
            }
            if ty.module_path == "serde_json" && ty.name == "Number" {
                *datatype = self.number.clone();
                return;
            }
            if ["std::result", "core::result"].contains(&ty.module_path.as_ref())
                && ty.name == "Result"
                && let NamedReferenceType::Inline { dt, .. } = &mut reference.inner
            {
                normalize_result(dt);
            }
        }
        match datatype {
            DataType::List(list) => self.normalize(&mut list.ty),
            DataType::Map(map) => {
                self.normalize(map.key_ty_mut());
                self.normalize(map.value_ty_mut());
            }
            DataType::Nullable(inner) => self.normalize(inner),
            DataType::Tuple(tuple) => tuple.elements.iter_mut().for_each(|ty| self.normalize(ty)),
            DataType::Intersection(types) => types.iter_mut().for_each(|ty| self.normalize(ty)),
            DataType::Struct(body) => self.fields(&mut body.fields),
            DataType::Enum(body) => body
                .variants
                .iter_mut()
                .for_each(|(_, variant)| self.fields(&mut variant.fields)),
            DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
                NamedReferenceType::Inline { dt, .. } => self.normalize(dt),
                NamedReferenceType::Reference { generics, .. } => {
                    generics.iter_mut().for_each(|(_, ty)| self.normalize(ty))
                }
                NamedReferenceType::Recursive(_) => {}
            },
            _ => {}
        }
    }

    fn fields(&self, fields: &mut Fields) {
        match fields {
            Fields::Named(fields) => fields.fields.iter_mut().for_each(|(_, field)| {
                if let Some(ty) = &mut field.ty {
                    self.normalize(ty);
                }
            }),
            Fields::Unnamed(fields) => fields.fields.iter_mut().for_each(|field| {
                if let Some(ty) = &mut field.ty {
                    self.normalize(ty);
                }
            }),
            Fields::Unit => {}
        }
    }
}

fn normalize_result(datatype: &mut DataType) {
    let DataType::Struct(body) = datatype else {
        panic!("unexpected Specta Result definition")
    };
    let Fields::Named(fields) = &body.fields else {
        panic!("unexpected Specta Result fields")
    };
    let mut result = Enum::default();
    for (name, field) in &fields.fields {
        let tag = match name.as_ref() {
            "ok" => "Ok",
            "err" => "Err",
            _ => panic!("unexpected Specta Result field"),
        };
        result.variants.push((
            tag.into(),
            Variant::unnamed()
                .field(Field::new(field.ty.clone().expect("Result field type")))
                .build(),
        ));
    }
    *datatype = result.into();
}
