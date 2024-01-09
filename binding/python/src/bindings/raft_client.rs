use pyo3::prelude::*;
use raftify::{
    create_client,
    raft::eraftpb::{ConfChangeV2, Message},
    Channel, RaftServiceClient,
};

use super::{
    errors::WrongArgumentError,
    raft_facade::TOKIO_RT,
    raft_rs::eraftpb::{conf_change_v2::PyConfChangeV2, message::PyMessage},
};

#[derive(Clone, Debug)]
enum Arguments {
    ChangeConfig { conf_change: ConfChangeV2 },
    SendMessage { message: Message },
    Propose { proposal: Vec<u8> },
    Empty,
}

#[derive(Clone)]
#[pyclass(name = "RaftServiceClient")]
pub struct PyRaftServiceClient {
    inner: RaftServiceClient<Channel>,
    args: Arguments,
}

#[pymethods]
impl PyRaftServiceClient {
    #[staticmethod]
    pub async fn build(addr: String) -> Self {
        let addr = addr.to_owned();
        let inner = PyRaftServiceClient::_create_client(addr).await;

        Self {
            inner,
            args: Arguments::Empty,
        }
    }

    pub fn prepare_conf_change(&mut self, conf_change: PyConfChangeV2) {
        self.args = Arguments::ChangeConfig {
            conf_change: conf_change.inner,
        };
    }

    // TODO: Defines the return type
    pub async fn change_config(&mut self) -> PyResult<(i32, Vec<u8>)> {
        match &self.args {
            Arguments::ChangeConfig { conf_change } => {
                let result = self
                    .inner
                    .change_config(conf_change.clone())
                    .await
                    .unwrap()
                    .into_inner();

                return Ok((result.result_type, result.data));
            }
            _ => {
                return Err(WrongArgumentError::new_err(
                    "Wrong argument type".to_string(),
                ))
            }
        }
    }

    pub fn prepare_send_message(&mut self, data: PyMessage) {
        self.args = Arguments::SendMessage {
            message: data.inner,
        };
    }

    pub async fn send_message(&mut self) -> PyResult<()> {
        match &self.args {
            Arguments::SendMessage { message } => {
                self.inner.send_message(message.clone()).await.unwrap();

                return Ok(());
            }
            _ => {
                return Err(WrongArgumentError::new_err(
                    "Wrong argument type".to_string(),
                ))
            }
        }
    }

    pub fn prepare_propose(&mut self, proposal: Vec<u8>) {
        self.args = Arguments::Propose { proposal };
    }

    pub async fn propose(&mut self) -> PyResult<()> {
        match &self.args {
            Arguments::Propose { proposal } => {
                self.inner
                    .propose(raftify::raft_service::ProposeArgs {
                        msg: proposal.clone(),
                    })
                    .await
                    .unwrap();

                return Ok(());
            }
            _ => {
                return Err(WrongArgumentError::new_err(
                    "Wrong argument type".to_string(),
                ))
            }
        }
    }

    pub async fn debug_node(&mut self) -> PyResult<String> {
        let response = self
            .inner
            .debug_node(raftify::raft_service::Empty {})
            .await
            .unwrap()
            .into_inner();

        return Ok(response.result_json);
    }

    pub async fn get_peers(&mut self) -> PyResult<String> {
        let response = self
            .inner
            .get_peers(raftify::raft_service::Empty {})
            .await
            .unwrap()
            .into_inner();

        return Ok(response.peers_json);
    }
}

impl PyRaftServiceClient {
    pub async fn _create_client(addr: String) -> RaftServiceClient<Channel> {
        TOKIO_RT.spawn(create_client(addr)).await.unwrap().unwrap()
    }
}
