use crate::response_message::{LocalResponseMsg, ResponseMessage, ServerResponseMsg};
use crate::{AbstractLogEntry, AbstractStateMachine};
use tokio::sync::oneshot;

pub(crate) enum ResponseSender<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine<LogEntry>> {
    Local(oneshot::Sender<LocalResponseMsg<LogEntry, FSM>>),
    Server(oneshot::Sender<ServerResponseMsg>),
}

impl<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine<LogEntry>>
    ResponseSender<LogEntry, FSM>
{
    pub fn send(self, response: ResponseMessage<LogEntry, FSM>) {
        match self {
            ResponseSender::Local(sender) => {
                if let ResponseMessage::Local(response) = response {
                    sender.send(response).unwrap()
                } else {
                    unreachable!()
                }
            }
            ResponseSender::Server(sender) => {
                if let ResponseMessage::Server(response) = response {
                    sender.send(response).unwrap()
                } else {
                    unreachable!()
                }
            }
        }
    }
}
