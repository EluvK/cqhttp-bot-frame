use regex::Regex;
use serde::{Deserialize, Serialize};

/// Used to check PostType.
#[derive(Debug, Serialize, Deserialize)]
pub struct CQPostMsg {
    pub post_type: CQPostMessageType,
    #[serde(skip)]
    _others: std::marker::PhantomData<CQPostMsg>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CQPostMessageType {
    Message,
    MetaEvent,
    Request,
    Notice,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CQMessageType {
    Group,
    Private,
}

#[derive(Debug, Serialize, Deserialize)]
struct Sender {
    age: i64,
    nickname: String,
    sex: String,
    user_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct CQPostMessageMsg {
    pub post_type: CQPostMessageType,
    pub message_type: CQMessageType,

    pub time: u64,    // timestamp
    pub self_id: u64, // self qq number
    pub user_id: u64, // sender qq number

    // sub_type: Option<String>,
    sender: Sender,
    pub message: String,
    pub raw_message: String,
    message_id: i64,
    font: i64,

    // for group message
    pub group_id: Option<u64>,

    // for private message
    target_id: Option<u64>, // self qq number
}

lazy_static::lazy_static! {
    static ref CQ_CODE_RE: Regex = Regex::new(r#"\[CQ:.*?\]"#).unwrap();
}

impl CQPostMessageMsg {
    pub fn parse_cq_code(mut self, bot_id: u64) -> (bool, Self) {
        let at_msg_re = Regex::new(format!("CQ:at,qq={}", bot_id).as_str()).unwrap();
        let mut is_at_msg = false;
        if at_msg_re.is_match(&self.raw_message) {
            is_at_msg = true;
        }
        self.message = CQ_CODE_RE.replace_all(&self.message, "").trim().to_string();
        (is_at_msg, self)
    }
}
