use pyo3::prelude::*;
use pyo3_stub_gen::{PyStubType, TypeInfo};
use statsig_rust::user::unit_id::UnitID;
use std::collections::HashSet;

#[derive(FromPyObject, IntoPyObject)]
pub enum UnitIdPy {
    Int(i64),
    Float(f64),
    String(String),
}

impl PyStubType for UnitIdPy {
    fn type_output() -> TypeInfo {
        TypeInfo {
            name: "typing.Union[builtins.str, builtins.int, builtins.float]".to_string(),
            import: HashSet::from(["builtins".into(), "typing".into()]),
        }
    }
}

impl UnitIdPy {
    pub fn into_unit_id(self) -> UnitID {
        match self {
            UnitIdPy::Int(i) => UnitID::Int(i),
            UnitIdPy::Float(f) => UnitID::Float(f),
            UnitIdPy::String(s) => UnitID::String(s),
        }
    }
}

impl From<UnitIdPy> for UnitID {
    fn from(value: UnitIdPy) -> Self {
        match value {
            UnitIdPy::Int(i) => UnitID::Int(i),
            UnitIdPy::Float(f) => UnitID::Float(f),
            UnitIdPy::String(s) => UnitID::String(s),
        }
    }
}
