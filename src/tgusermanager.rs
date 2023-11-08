use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum TgManagerCommand {
    #[allow(dead_code)]
    Get {
        user_id: String,
        reply_to: oneshot::Sender<String>,
    },
}

pub async fn user_manager(rx: &mut mpsc::Receiver<TgManagerCommand>) {
    log::info!("Request came");
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

#[cfg(test)]
mod tgusertest {
    use super::*;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn manager() {
        let (tx, mut rx) = mpsc::channel(32);
        let (response_tx, response_rx) = oneshot::channel();

        tokio::spawn(async move {
            user_manager(&mut rx).await;
        });

        tx.send(TgManagerCommand::Get {
            user_id: "0".to_string(),
            reply_to: response_tx,
        })
        .await
        .unwrap(); // Handle potential error properly

        // Now, await the response from the user_manager function
        if let Ok(response) = response_rx.await {
            println!("Received response: {}", response);
        } else {
            println!("The sender dropped without sending a response");
        }
    }
}
