// This bot throws a dice on each incoming message.

use crate::categories::{assign_category, CatStats};
use crate::convert::{convert, non_cat_items};
use crate::user::User;
use derive_more::From;
use qif_generator::{account::Account, account::AccountType};
use std::sync::atomic::{AtomicBool, Ordering};
use teloxide::types::*;
use teloxide::{net::Download, types::File as TgFile, Bot};
use teloxide::{prelude::*, utils::command::BotCommand};
use teloxide::{DownloadError, RequestError};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

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

/// Possible error while receiving a file
#[cfg(feature = "telegram")]
#[derive(Debug, Error, From)]
enum FileConvertError {
    /// Telegram request error
    #[error("JSON conversion error: {0}")]
    Request(String),
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
static IS_RUNNING: AtomicBool = AtomicBool::new(false);

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
async fn convert_file(
    jsonfile: &str,
    user: &mut User,
    ctx: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<String, FileConvertError> {
    let filepath = format!("{}.qif", jsonfile);
    log::info!("Converting file into {}", filepath);
    let mut file = File::create(&filepath).await?;
    log::info!("Got file");
    for i in non_cat_items(&jsonfile, &user) {
        log::info!("Message about {}", i);
        let newcat = input_category_from_tg(&i, &user.catmap);
        ctx.answer(format!("{} is set to {}", i, newcat)).await;
    }
    let acc = Account::new()
        .name("Wallet")
        .account_type(AccountType::Cash)
        .build();
    log::info!("Account is ready");
    let t = convert(jsonfile, "Test", user, &acc)?;
    log::info!("Conversion performed");
    file.write(acc.to_string().as_bytes()).await?;
    file.write(t.to_string().as_bytes()).await?;
    Ok(filepath)
}

#[cfg(feature = "telegram")]
pub fn bot_is_running() -> bool {
    IS_RUNNING.load(Ordering::SeqCst)
}

#[cfg(feature = "telegram")]
pub async fn input_category_from_tg(
    item: &str,
    categories: &CatStats,
    ctx: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> String {
    ctx.answer(format!("Input category for {}", item)).await;
    String::new()
}

#[cfg(feature = "telegram")]
async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting telegram bot");
    IS_RUNNING.store(true, Ordering::SeqCst);

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
                    if let Some(tguser) = message.update.from() {
                        let mut user = User::new(tguser.id, &None);
                        message
                            .answer(format!("Created user: {:} ", tguser.id))
                            .await?;
                        if let Ok(result) = convert_file(&newfile, &mut user, &message).await {
                            message
                                .answer(format!("File converted into: {:} ", result))
                                .await?;
                        }
                    }
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
    IS_RUNNING.store(false, Ordering::SeqCst);
}
