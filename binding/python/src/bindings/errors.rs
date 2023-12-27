use pyo3::{create_exception, exceptions::PyException, PyErr};

create_exception!(raftify, RaftError, PyException);
create_exception!(raftify, WrongArgumentError, PyException);
create_exception!(raftify, DecodingError, PyException);

#[inline]
pub fn runtime_error(msg: &str) -> PyErr {
    PyException::new_err(msg.to_string())
}
