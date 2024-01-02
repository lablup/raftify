use pyo3::{create_exception, exceptions::PyException, PyErr};

create_exception!(raftify, RaftError, PyException);
create_exception!(raftify, WrongArgumentError, PyException);
create_exception!(raftify, EncodingError, PyException);
create_exception!(raftify, DecodingError, PyException);
create_exception!(raftify, ApplyError, PyException);
create_exception!(raftify, SnapshotError, PyException);
create_exception!(raftify, RestoreError, PyException);

#[inline]
#[allow(dead_code)]
// Use it just for testing purposes
pub fn runtime_error(msg: &str) -> PyErr {
    PyException::new_err(msg.to_string())
}
