use crate::categories::CatStats;
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
    dispatching2::dialogue::{InMemStorage, Storage},
    macros::DialogueState,
    net::Download,
    prelude2::*,
    types::File as TgFile,
    utils::command::BotCommand,
    Bot, DownloadError, RequestError,
};
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
async fn download_file(
    downloader: &AutoSend<Bot>,
    file_id: &str,
) -> Result<String, FileReceiveError> {
    let TgFile {
        file_id, file_path, ..
    } = downloader.get_file(file_id).send().await?;
    log::info!("Attempt to download file");
    let filepath = format!("/tmp/{}", file_id);
    log::info!("Path: {}", filepath);
    let mut file = File::create(&filepath).await?;
    downloader.download_file(&file_path, &mut file).await?;
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
#[derive(DialogueState, Clone)]
#[handler_out(anyhow::Result<()>)]
pub enum State {
    #[handler(handle_idle)]
    Idle,

    #[handler(handle_json)]
    NewJson { filename: String },

    #[handler(handle_category)]
    CategorySelect { filename: String, item: String },

    #[handler(handle_subcategory)]
    SubCategorySelect {
        filename: String,
        item: String,
        category: String,
    },

    #[handler(handle_item_ready)]
    ItemReady {
        filename: String,
        item: String,
        fullcat: String,
    },

    #[handler(handle_qif_ready)]
    Ready,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

type QIFDialogue = Dialogue<State, InMemStorage<State>>;

async fn handle_idle(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: QIFDialogue,
) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Upload your file").await?;
    dialogue
        .update(State::NewJson {
            filename: "test".to_string(),
        })
        .await?;
    Ok(())
}

async fn handle_json(
    bot: AutoSend<Bot>,
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
                file_id = String::from_str(&doc.document.file_id).unwrap_or("".to_string());
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
        let user = User::new(msg.chat.id, &None);
        bot.send_message(msg.chat.id, format!("Active user: {:} ", msg.chat.id))
            .await?;
        let filepath = format!("{}.qif", &newfile);
        log::info!("Received file {}", &filepath);
        let mut i = non_cat_items(&newfile, &user);
        if let Some(item) = i.pop() {
            log::info!("No category for {}", &item);
            bot.send_message(msg.chat.id, format!("Select category for {}", item))
                .await?;
            dialogue
                .update(State::CategorySelect {
                    filename: filename,
                    item: item,
                })
                .await?;
        } else {
            log::info!("Empty state 2");
        }
    }
    Ok(())
}

async fn handle_category(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: QIFDialogue,
    (filename, item): (String, String), // Available from `State::Idle`.
) -> anyhow::Result<()> {
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

    bot.send_message(msg.chat.id, format!("Input subcategory for {}", item))
        .reply_markup(ReplyMarkup::InlineKeyboard(keyboard))
        .await?;

    match msg.text() {
        Some(cat) => {
            dialogue
                .update(State::SubCategorySelect {
                    filename: filename,
                    item: item,
                    category: cat.to_string(),
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
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: QIFDialogue,
    (filename, item, category): (String, String, String), // Available from `State::Idle`.
) -> anyhow::Result<()> {
    match msg.text() {
        Some(subcategory) => {
            bot.send_message(msg.chat.id, "Item ready").await?;
            dialogue
                .update(State::ItemReady {
                    filename: filename,
                    item: item,
                    fullcat: format!("{}:{}", category, subcategory),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me a subcategory.")
                .await?;
        }
    }
    Ok(())
}

async fn handle_item_ready(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: QIFDialogue,
    (filename, item, fullcat): (String, String, String), // Available from `State::Idle`.
) -> anyhow::Result<()> {
    bot.send_message(
        msg.chat.id,
        format!("Item {} is ready for caterogy {}", item, fullcat),
    )
    .await?;
    dialogue.update(State::Ready).await?;
    Ok(())
}

async fn handle_qif_ready(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: QIFDialogue,
) -> anyhow::Result<()> {
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

async fn callback_handler(q: CallbackQuery, bot: AutoSend<Bot>) -> anyhow::Result<()> {
    if let Some(version) = q.data {
        let text = format!("You chose: {}", version);

        match q.message {
            Some(Message { id, chat, .. }) => {
                bot.edit_message_text(chat.id, id, text).await?;
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
    teloxide::enable_logging!();
    log::info!("Starting telegram bot");
    IS_RUNNING.store(true, Ordering::SeqCst);
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move { user_manager(&mut rx).await });

    //    let storage = InMemStorage::new();

    let bot = Bot::from_env().auto_send();
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .dispatch_by::<State>(),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
    /*
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
    */
    drop(manager);
    IS_RUNNING.store(false, Ordering::SeqCst);
}
