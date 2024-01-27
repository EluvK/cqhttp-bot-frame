use crate::cq_msg::{recv::CQPostMessageMsg, send::CQSendMsg};

// msg that should handle
#[derive(Debug)]
pub struct RecvMsg {
    pub from_id: u64,
    pub content: String,
    pub group_id: Option<u64>,
}

impl RecvMsg {
    pub fn reply(&self, content: String) -> SendMsg {
        SendMsg {
            content,
            replay_id: Some(self.from_id.clone()),
            group_id: self.group_id,
        }
    }
}

// msg that handle reply
#[derive(Debug)]
pub struct SendMsg {
    pub content: String,
    pub replay_id: Option<u64>, // will be at in group
    pub group_id: Option<u64>,
}

impl From<CQPostMessageMsg> for RecvMsg {
    fn from(value: CQPostMessageMsg) -> Self {
        RecvMsg {
            from_id: value.user_id,
            content: value.message,
            group_id: value.group_id,
        }
    }
}

impl TryFrom<SendMsg> for CQSendMsg {
    type Error = anyhow::Error;

    fn try_from(value: SendMsg) -> Result<Self, Self::Error> {
        match (value.group_id, value.replay_id) {
            (Some(group_id), user_id) => {
                Ok(CQSendMsg::new_group_msg(group_id, user_id, value.content))
            }
            (None, Some(user_id)) => Ok(CQSendMsg::new_private_msg(user_id, value.content)),
            (None, None) => Err(anyhow::anyhow!("missing group id and user id")),
        }
    }
}
