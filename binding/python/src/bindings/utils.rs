use pyo3::{prelude::*, types::PyList};

pub fn get_python_repr(obj: &PyAny) -> String {
    obj.call_method("__repr__", (), None).unwrap().to_string()
}
