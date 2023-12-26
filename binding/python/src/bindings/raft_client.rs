use crate::bindings::raft_rs::eraftpb::conf_change_v2::PyConfChangeV2;
use pyo3::prelude::*;
use raftify::{create_client, raft, Channel, RaftServiceClient};
use tonic::Request;

#[derive(Clone)]
#[pyclass(name = "RaftClient")]
pub struct PyRaftClient {
    inner: RaftServiceClient<Channel>,
    conf_change: Option<PyConfChangeV2>,
}

#[pymethods]
impl PyRaftClient {
    #[staticmethod]
    pub async fn new(addr: String) -> Self {
        Self::_new(&addr)
    }

    pub fn prepare_conf_change(&mut self, conf_change: PyConfChangeV2) {
        self.conf_change = Some(conf_change);
    }

    // TODO: Defines the return type
    pub async fn change_config(&mut self) -> PyResult<(i32, Vec<u8>)> {
        let result = self
            .inner
            .change_config(Request::new(self.conf_change.take().unwrap().inner))
            .await
            .unwrap()
            .into_inner();

        return Ok((result.result_type, result.data));
    }

    // pub async fn send_message(&mut self) -> PyResult<()> {
    //     let result = self
    //         .inner
    //         .send_message()
    //         .await
    //         .unwrap()
    //         .into_inner();

    //     return Ok(());
    // }
}

impl PyRaftClient {
    #[tokio::main]
    pub async fn _new(addr: &str) -> Self {
        let addr = addr.to_owned();
        let inner = create_client(&addr).await.unwrap();

        Self {
            inner,
            conf_change: None,
        }
    }
}
