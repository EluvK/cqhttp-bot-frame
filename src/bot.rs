
use async_trait::async_trait;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    cq_msg::{
        recv::{CQMessageType, CQPostMessageMsg, CQPostMessageType, CQPostMsg},
        send::CQSendMsg,
    },
    msg::{RecvMsg, SendMsg},
};

#[derive(Debug, Deserialize)]
pub struct BotConfig {
    websocket: String,
    bot_qq: u64,
    root_qq: u64,
}

pub struct Bot<CmdType: Parser + Send + Sync> {
    config: BotConfig,
    handler: Arc<(dyn Handler<Cmd = CmdType> + Send + Sync)>,
    instant_rx: Receiver<SendMsg>,
}

// get msg from recv_rx, send msg to send_rx
async fn handle<CmdType: Parser + Send + Sync + 'static>(
    handler: Arc<(dyn Handler<Cmd = CmdType> + Send + Sync)>,
    mut recv_rx: Receiver<RecvMsg>,
    send_rx: Sender<SendMsg>,
    mut instant_rx: Receiver<SendMsg>,
) {
    loop {
        tokio::select! {
            Some(recv_msg) = recv_rx.recv() => {
                let send_rx = send_rx.clone();
                let handler = handler.clone();
                tokio::task::spawn(async move {
                    if let Some(send_msg) = match parse_cmd::<CmdType>(&recv_msg.content) {
                        Some(cmd) => {
                            if handler.check_cmd_auth(&cmd, &recv_msg) {
                                handler.handle_cmd(cmd).await
                            } else {
                                None
                            }
                        }
                        None => handler.handle_msg(recv_msg).await,
                    } {
                        send_rx.send(send_msg).await;
                    }
                });
            }
            Some(instant_msg) = instant_rx.recv() => {
                send_rx.send(instant_msg).await;
            }

        }
    }
}

fn parse_cmd<CmdType: Parser>(raw_msg: &str) -> Option<CmdType> {
    println!("parse:{raw_msg}");
    if let Some(raw_msg) = raw_msg.strip_prefix('#') {
        let mut cmds: Vec<&str> = raw_msg.split_whitespace().collect();
        cmds.insert(0, "");
        return CmdType::try_parse_from(cmds).ok();
    }
    None
}

#[async_trait]
pub trait Handler {
    type Cmd: Parser;
    async fn handle_msg(&self, msg: RecvMsg) -> Option<SendMsg>;
    async fn handle_cmd(&self, cmd: Self::Cmd) -> Option<SendMsg>;
    fn check_cmd_auth(&self, cmd: &Self::Cmd, ori_msg: &RecvMsg) -> bool;
}

impl<CmdType: Parser + Send + Sync + 'static> Bot<CmdType> {
    pub async fn new(
        config: BotConfig,
        handler: Arc<dyn Handler<Cmd = CmdType> + Send + Sync>,
        instant_rx: Receiver<SendMsg>,
    ) -> Self {
        Self {
            config,
            handler: handler.clone(),
            instant_rx,
        }
    }

    pub async fn start(self) -> ! {
        let (web_socket_stream, _) = connect_async(&self.config.websocket)
            .await
            .expect("Failed to connect to cphttp");

        let (recv_tx, recv_rx) = channel::<RecvMsg>(10);
        let (send_tx, mut send_rx) = channel::<SendMsg>(10);

        // cq-http -> socket -> recv_tx ... recv_rx
        // send_tx ... send_rx -> socket -> cq_http
        tokio::spawn({
            // let instant_tx = self.instant_rx.clone();
            let handler = self.handler.clone();
            async move {
                handle(handler, recv_rx, send_tx, self.instant_rx).await;
            }
        });

        let (mut ws_sender, mut ws_receiver) = web_socket_stream.split();
        // let interval = tokio::time::interval(Duration::from_millis(1000));
        loop {
            tokio::select! {
                // cq-http -> bot
                Some(Ok(Message::Text(msg))) = ws_receiver.next() => {
                    println!("bot frame recv from cq msg: {msg:?}");
                    if let Some(recv_msg) = analyzer_msg(msg, self.config.bot_qq) {
                        recv_tx.send(recv_msg).await.unwrap();
                    }
                }
                // bot -> cq-http
                Some(send_msg) = send_rx.recv() => {
                    println!("bot frame send msg to cq: {send_msg:?}");
                    if let Ok(send_msg) = CQSendMsg::try_from(send_msg) {
                        let reply = Message::Text(serde_json::to_string(&send_msg).unwrap());
                        ws_sender.send(reply).await.unwrap();
                    }
                }
            }
        }
    }
}

// determine should handle this message
fn analyzer_msg(msg: String, bot_qq: u64) -> Option<RecvMsg> {
    if let Ok(msg_type) = serde_json::from_str::<CQPostMsg>(msg.as_str()) {
        match msg_type.post_type {
            CQPostMessageType::Message => {
                let msg = serde_json::from_str::<CQPostMessageMsg>(&msg).ok()?;
                let (is_at, msg) = msg.parse_cq_code(bot_qq);
                println!("is_at: {is_at} msg.message_type:{:?}", msg.message_type);
                match (&msg.message_type, is_at) {
                    (CQMessageType::Private, _) | (CQMessageType::Group, true) => {
                        Some(RecvMsg::from(msg))
                    }
                    (CQMessageType::Group, false) => None,
                }
            }
            // for now only care Message.
            _rest => None,
        }
    } else {
        println!("error recv cq_http raw message: {msg:?}");
        None
    }
}