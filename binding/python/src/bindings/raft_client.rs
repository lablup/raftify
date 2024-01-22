use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use raftify::{create_client, Channel, RaftServiceClient};

use super::raft_rs::eraftpb::{conf_change_v2::PyConfChangeV2, message::PyMessage};

#[derive(Clone)]
#[pyclass(name = "RaftServiceClient")]
pub struct PyRaftServiceClient {
    inner: RaftServiceClient<Channel>,
}

#[pymethods]
impl PyRaftServiceClient {
    #[staticmethod]
    pub fn build<'a>(addr: String, py: Python<'a>) -> PyResult<&'a PyAny> {
        let addr = addr.to_owned();

        future_into_py(py, async move {
            let inner = create_client(addr).await.unwrap();
            Ok(Self { inner })
        })
    }

    // TODO: Defines the return type
    pub fn change_config<'a>(
        &'a mut self,
        conf_change: PyConfChangeV2,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let mut client = self.inner.clone();

        future_into_py(py, async move {
            let result = client
                .change_config(conf_change.inner)
                .await
                .unwrap()
                .into_inner();
            Ok((result.result_type, result.data))
        })
    }

    pub fn send_message<'a>(
        &'a mut self,
        message: PyMessage,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let mut client = self.inner.clone();

        future_into_py(py, async move {
            let _ = client
                .send_message(message.inner)
                .await
                .unwrap()
                .into_inner();
            Ok(())
        })
    }

    pub fn propose<'a>(&'a mut self, proposal: Vec<u8>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut client = self.inner.clone();

        future_into_py(py, async move {
            let _ = client
                .propose(raftify::raft_service::ProposeArgs { msg: proposal })
                .await
                .unwrap()
                .into_inner();
            Ok(())
        })
    }

    pub fn debug_node<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut client = self.inner.clone();

        future_into_py(py, async move {
            let response = client
                .debug_node(raftify::raft_service::Empty {})
                .await
                .unwrap()
                .into_inner();
            Ok(response.result_json)
        })
    }

    pub fn get_peers<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut client = self.inner.clone();

        future_into_py(py, async move {
            let response = client
                .get_peers(raftify::raft_service::Empty {})
                .await
                .unwrap()
                .into_inner();
            Ok(response.peers_json)
        })
    }
}
