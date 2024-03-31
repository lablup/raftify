use tokio::sync::oneshot;

use crate::{
    response::{
        local_response_message::LocalResponseMsg, server_response_message::ServerResponseMsg,
        ResponseMessage,
    },
    AbstractLogEntry, AbstractStateMachine,
};

pub(crate) enum ResponseSender<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    Local(oneshot::Sender<LocalResponseMsg<LogEntry, FSM>>),
    Server(oneshot::Sender<ServerResponseMsg>),
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> ResponseSender<LogEntry, FSM> {
    pub fn send(self, response: ResponseMessage<LogEntry, FSM>) {
        match self {
            ResponseSender::Local(tx_local) => {
                if let ResponseMessage::Local(response) = response {
                    tx_local.send(response).unwrap()
                } else {
                    unreachable!()
                }
            }
            ResponseSender::Server(tx_server) => {
                if let ResponseMessage::Server(response) = response {
                    tx_server.send(response).unwrap()
                } else {
                    unreachable!()
                }
            }
        }
    }
}
