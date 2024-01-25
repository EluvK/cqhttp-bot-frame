use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum CQSendMsgAction {
    SendPrivateMsg,
    SendGroupMsg,
}

#[derive(Debug, Serialize)]
struct CQSendMsgParams {
    user_id: Option<u64>,
    group_id: Option<u64>,
    message: String,
}

#[derive(Debug, Serialize)]
pub struct CQSendMsg {
    action: CQSendMsgAction,
    params: CQSendMsgParams,
}

impl CQSendMsg {
    pub fn new_private_msg(user_id: u64, message: String) -> Self {
        Self {
            action: CQSendMsgAction::SendPrivateMsg,
            params: CQSendMsgParams {
                user_id: Some(user_id),
                group_id: None,
                message,
            },
        }
    }

    pub fn new_group_msg(group_id: u64, user_id: Option<u64>, message: String) -> Self {
        let message = match user_id {
            Some(user_id) => format!("[CQ:at,qq={user_id}] {message}"),
            None => message,
        };
        Self {
            action: CQSendMsgAction::SendGroupMsg,
            params: CQSendMsgParams {
                user_id: None,
                group_id: Some(group_id),
                message,
            },
        }
    }
}
