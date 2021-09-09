// This bot throws a dice on each incoming message.

use crate::categories::{get_category_from_tg, CatStats};
use crate::convert::{convert, non_cat_items};
use crate::tgusermanager::{user_manager, TgManagerCommand};
use crate::user::User;

use derive_more::From;
use qif_generator::{account::Account, account::AccountType};
use std::sync::atomic::{AtomicBool, Ordering};
use teloxide::types::*;
use teloxide::{
    dispatching::dialogue::{InMemStorage, Storage},
    DownloadError, RequestError,
};
use teloxide::{net::Download, types::File as TgFile, Bot};
use teloxide::{prelude::*, utils::command::BotCommand};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};

#[cfg(feature = "telegram")]
#[tokio::main]
pub async fn bot() {
    run().await;
}

/// Possible error while receiving a file
#[cfg(feature = "telegram")]
#[derive(Debug, Error, From)]
enum FileReceiveError {
    /// Telegram request error
    #[error("Web request error: {0}")]
    Request(#[source] RequestError),
    /// Io error while writing file
    #[error("An I/O error: {0}")]
    Io(#[source] std::io::Error),
    /// Download error while getting file from telegram
    #[error("File download error: {0}")]
    Download(#[source] DownloadError),
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

    #[command(description = "Delete account.")]
    Delete,

    #[command(description = "Request something")]
    Request,
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
    for i in non_cat_items(jsonfile, user) {
        log::info!("Message about {}", i);
        let newcat = input_category_from_tg(&i, &user.catmap, &user.accounts, ctx).await;
        ctx.answer(format!("{} is set to {}", i, newcat))
            .await
            .unwrap();
    }
    let acc = Account::new()
        .name("Wallet")
        .account_type(AccountType::Cash)
        .build();

    let cat = &|item: &str, stats: &mut CatStats, accounts: &[String]| -> String {
        get_category_from_tg(item, stats, accounts, ctx)
    };
    let t = convert(jsonfile, "Test", user, &acc, cat)?;
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
    _cats: &CatStats,
    accounts: &[String],
    ctx: &UpdateWithCx<AutoSend<Bot>, Message>,
) -> String {
    log::info!("{:?}", accounts);
    let keyboard = InlineKeyboardMarkup::default().append_row(
        accounts
            .iter()
            .filter(|l| l.starts_with("Expenses:"))
            .map(|line| {
                InlineKeyboardButton::new(
                    line.strip_prefix("Expenses:").unwrap(),
                    InlineKeyboardButtonKind::CallbackData(line.into()),
                )
            }),
    );
    ctx.answer(format!("Input category for {}", item))
        .reply_markup(ReplyMarkup::InlineKeyboard(keyboard))
        .await
        .unwrap();
    String::new()
}

#[derive(Transition, From)]
pub enum Dialogue {
    NewJson(NewJsonState),
    CategorySelect(CategorySelectState),
    SubCategorySelect(SubCategorySelectState),
    Ready(QIFReadyState),
}

impl Default for Dialogue {
    fn default() -> Self {
        Self::NewJson(NewJsonState)
    }
}

pub struct NewJsonState;

pub struct CategorySelectState {
    pub item: String,
}

pub struct SubCategorySelectState {
    pub item: String,
}

pub struct QIFReadyState;

#[teloxide(subtransition)]
async fn new_json(
    state: NewJsonState,
    cx: TransitionIn<AutoSend<Bot>>,
    file_id: String,
) -> TransitionOut<Dialogue> {
    if let Ok(newfile) = download_file(cx.requester.inner(), &file_id).await {
        cx.answer(format!("File received: {:} ", newfile)).await?;
        if let Some(tguser) = cx.update.from() {
            let mut user = User::new(tguser.id, &None);
            cx.answer(format!("Created user: {:} ", tguser.id)).await?;
            if let Ok(result) = convert_file(&newfile, &mut user, &cx).await {
                cx.answer(format!("File converted into: {:} ", result))
                    .await?;
                next(CategorySelectState { item: file_id })
            } else {
                next(state)
            }
        } else {
            next(state)
        }
    } else {
        cx.answer("Waiting for a JSON receipt").await?;
        next(state)
    }
}

#[teloxide(subtransition)]
async fn category_select(
    state: CategorySelectState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!("Selecting category for {}", item))
        .await?;
    next(state)
}

#[teloxide(subtransition)]
async fn subcategory_select(
    state: SubCategorySelectState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!("Selecting subcategory for {}", item))
        .await?;
    next(state)
}

#[teloxide(subtransition)]
async fn subcategory_select(
    state: QIFReadyState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!("QIF is ready for {}", item)).await?;
    next(state)
}

type StorageError = <InMemStorage<Dialogue> as Storage<Dialogue>>::Error;

#[derive(Debug, Error)]
enum Error {
    #[error("error from Telegram: {0}")]
    TelegramError(#[from] RequestError),
}

type In = DialogueWithCx<AutoSend<Bot>, Message, Dialogue, StorageError>;

async fn handle_message(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
    dialogue: Dialogue,
    tx: mpsc::Sender<TgManagerCommand>,
) -> TransitionOut<Dialogue> {
    match cx.update.text().map(ToOwned::to_owned) {
        None => {
            let mut is_file = false;
            let mut file_id: String = "".to_string();
            {
                let update = &cx.update;
                if let MessageKind::Common(msg) = &update.kind {
                    if let MediaKind::Document(doc) = &msg.media_kind {
                        is_file = true;
                        file_id = doc.document.file_id.clone();
                    }
                }
            }
            if is_file {
                Ok(dialogue.react(cx, file_id).await?)
            } else {
                next(dialogue)
            }
        }
        Some(ans) => {
            if let Ok(command) = Command::parse(&ans, "tgqif") {
                match command {
                    Command::Help => {
                        cx.answer(Command::descriptions()).send().await?;
                    }
                    Command::Start => {
                        if let Some(user) = cx.update.from() {
                            cx.answer(format!(
                                "You registered as @{} with id {}.",
                                user.first_name, user.id
                            ))
                            .await?;
                        }
                    }
                    Command::Delete => {
                        if let Some(user) = cx.update.from() {
                            cx.answer(format!("Deleting data for user {}", user.id))
                                .await?;
                        }
                    }
                    Command::Request => {
                        let (send, recv) = oneshot::channel();
                        if tx
                            .send(TgManagerCommand::Get {
                                user_id: ans,
                                reply_to: send,
                            })
                            .await
                            .is_err()
                        {
                            cx.answer("Can't request data").await?;
                        };

                        match recv.await {
                            Ok(value) => cx.answer(format!("I have an answer: {} ", value)).await?,
                            Err(_) => cx.answer("No data available").await?,
                        };
                    }
                }
            }
            next(dialogue)
        }
    }
}

#[cfg(feature = "telegram")]
async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting telegram bot");
    IS_RUNNING.store(true, Ordering::SeqCst);
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move { user_manager(&mut rx).await });

    let bot = Bot::from_env().auto_send();
    // TODO: Add Dispatcher to process UpdateKinds
    Dispatcher::new(bot)
        .messages_handler(DialogueDispatcher::with_storage(
            move |DialogueWithCx { cx, dialogue }: In| {
                let _tx = tx.clone();
                async move {
                    let dialogue = dialogue.expect("std::convert::Infallible");
                    handle_message(cx, dialogue, _tx)
                        .await
                        .expect("Something wrong with the bot!")
                }
            },
            InMemStorage::new(),
        ))
        .dispatch()
        .await;
    drop(manager);
    IS_RUNNING.store(false, Ordering::SeqCst);
}
