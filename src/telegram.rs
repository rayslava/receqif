// This bot throws a dice on each incoming message.

use derive_more::From;
use teloxide::types::*;
use teloxide::{net::Download, types::File as TgFile, Bot};
use teloxide::{prelude::*, utils::command::BotCommand};
use teloxide::{DownloadError, RequestError};
use thiserror::Error;
use tokio::fs::File;

#[cfg(feature = "telegram")]
#[tokio::main]
pub async fn bot() {
    run().await;
}

/// Possible error while receiving a file
#[cfg(feature = "telegram")]
#[derive(Debug, Error, From)]
enum FileReceiveError {
    /// Download process error
    #[error("File download error: {0}")]
    Download(#[source] DownloadError),
    /// Telegram request error
    #[error("Web request error: {0}")]
    Request(#[source] RequestError),
    /// Io error while writing file
    #[error("An I/O error: {0}")]
    Io(#[source] std::io::Error),
}

#[derive(BotCommand, Debug)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Register new user in bot.")]
    Start,
}

#[cfg(feature = "telegram")]
async fn download_file(downloader: &Bot, file_id: &str) -> Result<String, FileReceiveError> {
    let TgFile {
        file_id, file_path, ..
    } = downloader.get_file(file_id).send().await?;
    let filepath = format!("/tmp/{}", file_id);
    let mut file = File::create(&filepath).await?;
    downloader.download_file(&file_path, &mut file).await?;
    Ok(filepath)
}

#[cfg(feature = "telegram")]
async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting dices_bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repl(bot, |message| async move {
        let update = &message.update;
        if let MessageKind::Common(msg) = &update.kind {
            if let MediaKind::Document(doc) = &msg.media_kind {
                if let Ok(newfile) =
                    download_file(&message.requester.inner(), &doc.document.file_id).await
                {
                    message
                        .answer(format!("File received: {:} ", newfile))
                        .await?;
                }

                message.answer_dice().await?;
            } else if let Some(line) = message.update.text() {
                if let Ok(command) = Command::parse(line, "tgqif") {
                    match command {
                        Command::Help => {
                            message.answer(Command::descriptions()).send().await?;
                        }
                        Command::Start => {
                            if let Some(user) = message.update.from() {
                                message
                                    .answer(format!(
                                        "You registered as @{} with id {}.",
                                        user.first_name, user.id
                                    ))
                                    .await?;
                            }
                        }
                    }
                }
            }
        }
        respond(())
    })
    .await;
}
