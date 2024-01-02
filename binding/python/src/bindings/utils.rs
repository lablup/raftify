use pyo3::{prelude::*, types::PyList};

pub fn get_python_repr(obj: &PyAny) -> String {
    obj.call_method("__repr__", (), None).unwrap().to_string()
}

pub fn new_py_list<T, U>(
    py: Python<'_>,
    elements: impl IntoIterator<Item = T, IntoIter = U>,
) -> PyResult<&PyList>
where
    T: ToPyObject,
    U: ExactSizeIterator<Item = T>,
{
    let items = elements
        .into_iter()
        .map(|e| e.to_object(py))
        .collect::<Vec<_>>();
    Ok(PyList::new(py, &items))
}
