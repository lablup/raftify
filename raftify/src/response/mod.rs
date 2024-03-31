use std::marker::PhantomData;

use crate::{AbstractLogEntry, AbstractStateMachine};

use self::{local_response_message::LocalResponseMsg, server_response_message::ServerResponseMsg};

pub mod local_response_message;
pub mod server_response_message;

pub enum ResponseMessage<LogEntry: AbstractLogEntry, FSM: AbstractStateMachine> {
    Server(ServerResponseMsg),
    Local(LocalResponseMsg<LogEntry, FSM>),
    _Phantom(PhantomData<LogEntry>),
}
