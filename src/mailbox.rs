use crate::error::{Error, Result};
use crate::request_message::RequestMessage;
use crate::response_message::ResponseMessage;

use raft::eraftpb::{ConfChangeSingle, ConfChangeType, ConfChangeV2};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use std::collections::HashMap;
use std::time::Duration;

/// A mailbox to send messages to a running raft node.
#[derive(Clone)]
pub struct Mailbox {
    pub snd: mpsc::Sender<RequestMessage>,
    pub peers: HashMap<u64, String>,
    pub logger: slog::Logger,
}

impl Mailbox {
    pub async fn send(&self, message: Vec<u8>) -> Result<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        let proposal = RequestMessage::Propose {
            proposal: message,
            chan: tx,
        };
        let sender = self.snd.clone();
        // TODO make timeout duration a variable
        match sender.send(proposal).await {
            Ok(_) => match timeout(Duration::from_secs(2), rx).await {
                Ok(Ok(ResponseMessage::Response { data })) => Ok(data),
                err => {
                    slog::trace!(self.logger, "Error: {:?}", err);
                    Err(Error::Unknown)
                }
            },
            err => {
                slog::trace!(self.logger, "Error: {:?}", err);
                Err(Error::Unknown)
            }
        }
    }

    pub async fn leave(&self) -> Result<()> {
        let mut conf_change = ConfChangeV2::default();
        let mut cs = ConfChangeSingle::default();
        cs.set_node_id(0);
        cs.set_change_type(ConfChangeType::RemoveNode);
        conf_change.set_changes(vec![cs].into());
        // conf_change.set_context(serialize(&self.addr)?);

        let sender = self.snd.clone();
        let (chan, rx) = oneshot::channel();
        match sender
            .send(RequestMessage::ConfigChange { conf_change, chan })
            .await
        {
            Ok(_) => match rx.await {
                Ok(ResponseMessage::Ok) => Ok(()),
                _ => Err(Error::Unknown),
            },
            _ => Err(Error::Unknown),
        }
    }
}
