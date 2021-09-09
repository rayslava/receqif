use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum TgManagerCommand {
    Get {
        user_id: String,
        reply_to: oneshot::Sender<String>,
    },
}

pub async fn user_manager(rx: &mut mpsc::Receiver<TgManagerCommand>) {
    log::info!("Reqest came");
    while let Some(cmd) = rx.recv().await {
        use TgManagerCommand::*;
        log::info!("Command received");

        match cmd {
            Get { user_id, reply_to } => {
                log::info!("{}", format!("Get command found, sending {}", user_id));
                reply_to
                    .send(format!("You've requested {}", user_id))
                    .unwrap();
            }
        }
    }
}
