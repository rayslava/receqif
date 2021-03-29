// This bot throws a dice on each incoming message.

use teloxide::prelude::*;
use teloxide::types::*;
use teloxide::{net::Download, types::File as TgFile, Bot};
use tokio::fs::File;

#[tokio::main]
pub async fn bot() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting dices_bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repl(bot, |message| async move {
        let update = &message.update;
        if let MessageKind::Common(msg) = &update.kind {
            if let MediaKind::Document(doc) = &msg.media_kind {
                log::info!("{:?}", &doc.document);
                if let Ok(TgFile {
                    file_id,
                    file_path,
                    file_size,
                    ..
                }) = &message
                    .requester
                    .get_file(&doc.document.file_id)
                    .send()
                    .await
                {
                    let filepath = format!("/tmp/{}", file_id);
                    if let Ok(mut file) = File::create(filepath).await {
                        if message
                            .requester
                            .download_file(&file_path, &mut file)
                            .await
                            .is_ok()
                        {
                            message
                                .answer(format!("File received: {:} bytes", file_size))
                                .await?;
                        }
                    }
                }

                message.answer_dice().await?;
            }
        }
        respond(())
    })
    .await;
}
