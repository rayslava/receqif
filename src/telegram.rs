use crate::categories::CatStats;
use crate::convert::{convert, non_cat_items};
use crate::tgusermanager::{user_manager, TgManagerCommand};
use crate::user::User;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;

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
    net::Download,
    prelude::*,
    types::File as TgFile,
    utils::command::BotCommands,
    Bot, DownloadError, RequestError,
};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::UnboundedReceiverStream;

// impl Into<i64> for ChatId {
//     fn into(self) -> i64 {
//         self.0
//     }
// }

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

#[derive(BotCommands, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
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
    let TgFile { path, .. } = downloader.get_file(file_id).send().await?;
    log::info!("Attempt to download file");
    let filepath = format!("/tmp/{}", file_id);
    log::info!("Path: {}", filepath);
    let mut file = File::create(&filepath).await?;
    downloader.download_file(&path, &mut file).await?;
    Ok(filepath)
}

/*
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
*/
#[cfg(feature = "telegram")]
pub fn bot_is_running() -> bool {
    IS_RUNNING.load(Ordering::SeqCst)
}
/*
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
 */

#[derive(Clone, Debug)]
pub enum State {
    Idle,

    NewJson {
        filename: String,
    },

    CategorySelect {
        filename: String,
        item: String,
        items_left: Vec<String>,
        items_processed: HashMap<String, String>,
    },

    SubCategorySelect {
        filename: String,
        item: String,
        category: String,
        items_left: Vec<String>,
        items_processed: HashMap<String, String>,
    },

    Ready,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            State::Idle => write!(f, "Idle"),
            State::NewJson { filename } => write!(f, "NewJson {}", filename),
            State::CategorySelect {
                filename,
                item,
                items_left,
                items_processed,
            } => {
                write!(f, "Category: {}, {}", filename, item)
            }
            State::SubCategorySelect {
                filename,
                item,
                category,
                items_left,
                items_processed,
            } => write!(f, "SubCategory: {}, {}, {}", filename, item, category),
            State::Ready => write!(f, "Ready"),
        }
    }
}

type QIFDialogue = Dialogue<State, InMemStorage<State>>;

async fn handle_idle(bot: Bot, msg: Message, dialogue: QIFDialogue) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Upload your file").await?;
    dialogue
        .update(State::NewJson {
            filename: "test".to_string(),
        })
        .await?;
    Ok(())
}

async fn handle_json(
    bot: Bot,
    msg: Message,
    dialogue: QIFDialogue,
    (filename,): (String,), // Available from `State::Idle`.
) -> anyhow::Result<()> {
    log::info!("File {}", &filename);
    let mut is_file = false;
    let mut file_id: String = "".to_string();
    {
        if let MessageKind::Common(msg) = &msg.kind {
            log::info!("It's message");
            if let MediaKind::Document(doc) = &msg.media_kind {
                is_file = true;
                file_id = String::from_str(&doc.document.file.id).unwrap_or("".to_string());
                log::info!("It's file with id {:}", file_id);
            }
        }
    }

    if is_file {
        log::info!("File {} received", file_id);
        bot.send_message(msg.chat.id, format!("New file received!!!111 {}", file_id))
            .await?;
    } else {
        bot.send_message(msg.chat.id, format!("Unsupported media provided"))
            .await?;
    }

    if let Ok(newfile) = download_file(&bot, &file_id).await {
        bot.send_message(msg.chat.id, format!("File received: {:} ", newfile))
            .await?;
        let user = User::new(msg.chat.id.0, &None);
        bot.send_message(msg.chat.id, format!("Active user: {:} ", msg.chat.id))
            .await?;
        let filepath = format!("{}.qif", &newfile);
        log::info!("Received file {}", &filepath);
        let mut i = non_cat_items(&newfile, &user);
        if let Some(item) = i.pop() {
            log::info!("No category for {}", &item);
            bot.send_message(
                msg.chat.id,
                format!("Input category to search for {}", item),
            )
            .await?;
            dialogue
                .update(State::CategorySelect {
                    filename,
                    item,
                    items_left: i,
                    items_processed: HashMap::new(),
                })
                .await?;
        } else {
            log::info!("Empty state 2");
        }
    }
    Ok(())
}

async fn handle_category(
    bot: Bot,
    msg: Message,
    dialogue: QIFDialogue,
    (filename, item, items_left, items_processed): (
        String,
        String,
        Vec<String>,
        HashMap<String, String>,
    ), // Available from `State::NewJson`.
) -> anyhow::Result<()> {
    let accounts = [
        "Expenses:Alco".to_string(),
        "Expenses:Groceries".to_string(),
    ];
    let userid = if let Some(user) = msg.from() {
        user.id.0
    } else {
        0
    };
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

    bot.send_message(msg.chat.id, format!("Input subcategory for {}", item))
        .reply_markup(ReplyMarkup::InlineKeyboard(keyboard))
        .await?;

    match msg.text() {
        Some(cat) => {
            dialogue
                .update(State::SubCategorySelect {
                    filename,
                    item,
                    category: cat.to_string(),
                    items_left,
                    items_processed,
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me a category.").await?;
        }
    }
    Ok(())
}

async fn handle_subcategory(
    bot: Bot,
    msg: Message,
    dialogue: QIFDialogue,
    (filename, item, category, mut items_left, mut items_processed): (
        String,
        String,
        String,
        Vec<String>,
        HashMap<String, String>,
    ), // Available from `State::Category`.
) -> anyhow::Result<()> {
    match msg.text() {
        Some(subcategory) => {
            bot.send_message(msg.chat.id, "Item ready").await?;
            items_processed.insert(item, category);
            if items_left.len() > 0 {
                if let Some(nextitem) = items_left.pop() {
                    dialogue
                        .update(State::CategorySelect {
                            filename,
                            item: nextitem,
                            items_left,
                            items_processed,
                        })
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, "Can't pop next item :(")
                        .await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me a subcategory.")
                .await?;
        }
    }
    Ok(())
}

async fn handle_qif_ready(bot: Bot, msg: Message, dialogue: QIFDialogue) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "QIF is ready.").await?;
    dialogue.update(State::Idle).await?;
    Ok(())
}
/*
type StorageError = <InMemStorage<QIFDialogue> as Storage<QIFDialogue>>::Error;

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
                let d = stor.get_dialogue(chat.id);
                d.next(d);
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
}*/

async fn callback_handler(q: CallbackQuery, bot: Bot, dialogue: QIFDialogue) -> anyhow::Result<()> {
    if let Some(version) = q.data {
        let text = format!("You chose: {}", version);

        match q.message {
            Some(Message { id, chat, .. }) => {
                bot.edit_message_text(chat.id, id, text.clone()).await?;
                let state = dialogue.get().await?;
                if let Some(data) = state {
                    log::info!("Data: {}", data);
                    if let State::SubCategorySelect {
                        filename,
                        item,
                        category,
                        items_left,
                        items_processed,
                    } = data
                    {
                        log::info!("SubCategory match!");
                        bot.send_message(
                            chat.id,
                            format!("Item {} is ready for caterogy {}", item, category),
                        )
                        .await?;
                        todo!("Here item is ready, we need to check for next one");
                        dialogue.update(State::Ready).await?;
                    } else {
                        log::info!("No SubCategory match!");
                    }
                }
            }
            None => {
                if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, text).await?;
                }
            }
        }

        log::info!("You chose: {}", version);
    }

    Ok(())
}

#[cfg(feature = "telegram")]
async fn run() {
    //    teloxide::enable_logging!();
    log::info!("Starting telegram bot");
    IS_RUNNING.store(true, Ordering::SeqCst);
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move { user_manager(&mut rx).await });

    //    let storage = InMemStorage::new();

    let bot = Bot::from_env();
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .branch(teloxide::handler![State::Idle].endpoint(handle_idle))
                // No idea about `{filename, }`, but otherwise thread "'tokio-runtime-worker' panicked at '(alloc::string::String,) was requested, but not provided."
                .branch(
                    #[rustfmt::skip]
		    teloxide::handler![State::NewJson { filename, }].endpoint(handle_json),
                )
                .branch(
                    teloxide::handler![State::CategorySelect {
                        filename,
                        item,
                        items_left,
                        items_processed,
                    }]
                    .endpoint(handle_category),
                )
                .branch(
                    teloxide::handler![State::SubCategorySelect {
                        filename,
                        item,
                        category,
                        items_left,
                        items_processed,
                    }]
                    .endpoint(handle_subcategory),
                )
                .branch(teloxide::handler![State::Ready].endpoint(handle_qif_ready)),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
                .endpoint(callback_handler),
        );
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    drop(manager);
    IS_RUNNING.store(false, Ordering::SeqCst);
}
