use crate::categories::{get_category_from_tg, CatStats};
use crate::convert::{convert, non_cat_items};
use crate::tgusermanager::{user_manager, TgManagerCommand};
use crate::user::User;
use std::error::Error as StdError;

use derive_more::From;
use qif_generator::{account::Account, account::AccountType};
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
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
use tokio_stream::wrappers::UnboundedReceiverStream;

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
    cx: &UpdateWithCx<AutoSend<Bot>, Message>,
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
    cx.answer(format!("Input category for {}", item))
        .reply_markup(ReplyMarkup::InlineKeyboard(keyboard))
        .await
        .unwrap();
    String::new()
}

#[derive(Transition, From, Clone)]
pub enum Dialogue {
    Idle(IdleState),
    NewJson(NewJsonState),
    CategorySelect(CategorySelectState),
    SubCategorySelect(SubCategorySelectState),
    ItemReady(ItemReadyState),
    Ready(QIFReadyState),
}

impl Default for Dialogue {
    fn default() -> Self {
        Self::Idle(IdleState)
    }
}

#[derive(Clone)]
pub struct IdleState;

#[derive(Clone)]
pub struct NewJsonState {
    pub filename: String,
}

#[derive(Clone)]
pub struct CategorySelectState {
    pub filename: String,
    pub item: String,
}

#[derive(Clone)]
pub struct SubCategorySelectState {
    pub filename: String,
    pub item: String,
    pub category: String,
}

#[derive(Clone)]
pub struct ItemReadyState {
    pub filename: String,
    pub item: String,
    pub fullcat: String,
}

#[derive(Clone)]
pub struct QIFReadyState;

#[teloxide(subtransition)]
async fn new_json(
    state: NewJsonState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    log::info!("File {}", &state.filename);
    let mut is_file = false;
    let mut file_id: String = "".to_string();
    {
        let update = &cx.update;
        if let MessageKind::Common(msg) = &update.kind {
            if let MediaKind::Document(doc) = &msg.media_kind {
                is_file = true;
                file_id = String::from_str(&state.filename).unwrap_or("".to_string());
            }
        }
    }
    if is_file {
        log::info!("File {} received", file_id);
        cx.answer(format!("New file received!!!111 {}", file_id))
            .await?;
    } else {
        cx.answer(format!("Unsupported media provided")).await?;
    }

    if let Ok(newfile) = download_file(cx.requester.inner(), &file_id).await {
        cx.answer(format!("File received: {:} ", newfile)).await?;
        if let Some(tguser) = cx.update.from() {
            let user = User::new(tguser.id, &None);
            cx.answer(format!("Active user: {:} ", tguser.id)).await?;
            let filepath = format!("{}.qif", &newfile);
            log::info!("Received file {}", &filepath);
            let mut i = non_cat_items(&newfile, &user);
            if let Some(item) = i.pop() {
                log::info!("No category for {}", &item);
                cx.answer(format!("Select category for {}", item)).await?;
                next(CategorySelectState {
                    filename: state.filename,
                    item,
                })
            } else {
                log::info!("Empty state");
                next(state)
            }

        /*            if let Ok(result) = convert_file(&newfile, &mut user, &cx).await {
                        cx.answer(format!("File converted into: {:} ", result))
                            .await?;
                        next(CategorySelectState { item: file_id })

                    } else {
                        next(state)
                    }
        */
        } else {
            log::info!("Empty state 2");
            next(state)
        }
    } else {
        log::info!("Newfile {} fail", item);
        cx.answer("Waiting for a JSON receipt in new_json").await?;
        next(state)
    }
}

#[teloxide(subtransition)]
async fn category_select(
    state: CategorySelectState,
    cx: TransitionIn<AutoSend<Bot>>,
    ans: String,
) -> TransitionOut<Dialogue> {
    let accounts = [
        "Expenses:Alco".to_string(),
        "Expenses:Groceries".to_string(),
    ];
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

    cx.answer(format!("Input category for {}", state.item))
        .reply_markup(ReplyMarkup::InlineKeyboard(keyboard))
        .await?;

    next(SubCategorySelectState {
        filename: state.filename,
        item: state.item,
        category: ans,
    })
}

#[teloxide(subtransition)]
async fn subcategory_select(
    state: SubCategorySelectState,
    cx: TransitionIn<AutoSend<Bot>>,
    subcategory: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!("Select subcategory for {}", state.item))
        .await?;
    next(ItemReadyState {
        filename: state.filename,
        item: state.item,
        fullcat: format!("{}:{}", state.category, subcategory),
    })
}

#[teloxide(subtransition)]
async fn item_ready(
    state: ItemReadyState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!(
        "Item {} is ready for caterogy {}",
        state.item, state.fullcat
    ))
    .await?;
    next(QIFReadyState)
}

#[teloxide(subtransition)]
async fn qif_ready(
    state: QIFReadyState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!("QIF is ready for {}", item)).await?;
    next(IdleState)
}

#[teloxide(subtransition)]
async fn idling(
    state: IdleState,
    cx: TransitionIn<AutoSend<Bot>>,
    item: String,
) -> TransitionOut<Dialogue> {
    cx.answer(format!("Waiting for json or command")).await?;
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
    let ans = cx.update.text().map(ToOwned::to_owned);
    match dialogue {
        Dialogue::Idle(_) => {
            match ans {
                None => {
                    log::info!("No text");
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
                        log::info!("File {} received", file_id);
                        next(NewJsonState { filename: file_id })
                    //	dialogue.react(cx, file_id).await
                    } else {
                        cx.answer(format!("Unsupported media provided")).await?;
                        next(dialogue)
                    }
                }
                Some(ans) => {
                    if let Ok(command) = Command::parse(&ans, "tgqif") {
                        match command {
                            Command::Help => {
                                cx.answer(Command::descriptions()).send().await?;
                                next(dialogue)
                            }
                            Command::Start => {
                                if let Some(user) = cx.update.from() {
                                    cx.answer(format!(
                                        "You registered as @{} with id {}.",
                                        user.first_name, user.id
                                    ))
                                    .await?;
                                }
                                next(dialogue)
                            }
                            Command::Delete => {
                                if let Some(user) = cx.update.from() {
                                    cx.answer(format!("Deleting data for user {}", user.id))
                                        .await?;
                                }
                                next(dialogue)
                            }
                            Command::Request => {
                                let (send, recv) = oneshot::channel();
                                if tx
                                    .send(TgManagerCommand::Get {
                                        user_id: ans.clone(),
                                        reply_to: send,
                                    })
                                    .await
                                    .is_err()
                                {
                                    cx.answer("Can't request data").await?;
                                };

                                match recv.await {
                                    Ok(value) => {
                                        cx.answer(format!("I have an answer: {} ", value)).await?
                                    }
                                    Err(_) => cx.answer("No data available").await?,
                                };
                                next(dialogue)
                            }
                        }
                    } else {
                        next(dialogue)
                    }
                }
            }
        }
        _ => dialogue.react(cx, ans.unwrap_or(String::new())).await, //next(dialogue)
                                                                     //	    dialogue.react(cx, ans).await
    }
}

/// When it receives a callback from a button it edits the message with all
/// those buttons writing a text with the selected Debian version.
async fn callback_handler(
    cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>,
    stor: Arc<InMemStorage<Dialogue>>,
) -> Result<(), Box<dyn StdError + Send + Sync>>
where
{
    let UpdateWithCx {
        requester: bot,
        update: query,
    } = cx;

    if let Some(version) = query.data {
        let text = format!("{}", version);

        match query.message {
            Some(Message { id, chat, .. }) => {
                //                bot.edit_message_text(chat.id, id, text).await?;
                bot.send_message(chat.id, text).await?;
            }
            None => {
                if let Some(id) = query.inline_message_id {
                    //                    bot.edit_message_text_inline(dbg!(id), text).await?;
                    bot.send_message(id, text).await?;
                }
            }
        }

        log::info!("You chose: {}", version);
    }

    Ok(())
}

#[cfg(feature = "telegram")]
async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting telegram bot");
    IS_RUNNING.store(true, Ordering::SeqCst);
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move { user_manager(&mut rx).await });

    let storage = InMemStorage::new();

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
            storage.clone(),
        ))
        .callback_queries_handler({
            move |rx: DispatcherHandlerRx<AutoSend<Bot>, CallbackQuery>| {
                UnboundedReceiverStream::new(rx).for_each_concurrent(None, {
                    move |cx| {
                        let storage = storage.clone();
                        async move {
                            callback_handler(cx, storage).await.log_on_error().await;
                        }
                    }
                })
            }
        })
        .dispatch()
        .await;
    drop(manager);
    IS_RUNNING.store(false, Ordering::SeqCst);
}
