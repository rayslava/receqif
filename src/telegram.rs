use crate::categories;
use crate::convert::{convert, non_cat_items};
use qif_generator::account::{Account, AccountType};

use crate::tgusermanager::user_manager;
use crate::user::User;
use std::collections::HashMap;
use std::fmt;

use derive_more::From;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use teloxide::types::*;
use teloxide::{
    dispatching::dialogue::InMemStorage, net::Download, prelude::*, types::File as TgFile,
    utils::command::BotCommands, Bot, DownloadError, RequestError,
};
use thiserror::Error;
use tokio::fs::File;
use tokio::sync::mpsc;

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

#[derive(BotCommands, Clone, Debug)]
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

    #[command(description = "Cancel processing of current file")]
    Cancel,
}

async fn command_handler(
    bot: Bot,
    dialogue: QIFDialogue,
    me: teloxide::types::Me,
    msg: Message,
    cmd: Command,
) -> HandlerResult {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Start => bot.send_message(msg.chat.id, format!("Starting")).await?,
        Command::Delete => bot.send_message(msg.chat.id, format!("Deleting")).await?,

        Command::Request => bot.send_message(msg.chat.id, format!("Requesting")).await?,
        Command::Cancel => {
            dialogue.update(State::Idle).await?;
            bot.send_message(msg.chat.id, format!("Dialogue state reset"))
                .await?
        }
    };

    Ok(())
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

    Ready {
        filename: String,
        item_categories: HashMap<String, String>,
    },
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
            State::Ready {
                filename,
                item_categories,
            } => write!(
                f,
                "Conversion is ready for file {} the following items: {:#?}",
                filename, item_categories
            ),
        }
    }
}

type QIFDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn handle_idle(bot: Bot, dialogue: QIFDialogue, msg: Message) -> HandlerResult {
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
    dialogue: QIFDialogue,
    msg: Message,
    (filename,): (String,), // Available from `State::Idle`.
) -> HandlerResult {
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
        bot.send_message(msg.chat.id, format!("Not supported file sent"))
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
                    filename: newfile,
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
    dialogue: QIFDialogue,
    msg: Message,
    (filename, item, items_left, items_processed): (
        String,
        String,
        Vec<String>,
        HashMap<String, String>,
    ), // Available from `State::NewJson`.
) -> HandlerResult {
    let version = msg.text();
    if version.is_none() {
        bot.send_message(msg.chat.id, format!("Input subcategory for {}", item))
            .await?;
        dialogue
            .update(State::CategorySelect {
                filename,
                item,
                items_left,
                items_processed,
            })
            .await?;
        return Ok(());
    };

    let version = version.unwrap();

    let user = User::new(msg.chat.id.0, &None);
    let accounts = user
        .accounts
        .iter()
        .filter(|&e| {
            e.starts_with("Expenses:") && e.to_lowercase().contains(&version.to_lowercase())
        })
        .collect::<Vec<_>>();

    if accounts.is_empty() {
        bot.send_message(msg.chat.id, format!("Input subcategory for {}", item))
            .await?;
        dialogue
            .update(State::CategorySelect {
                filename,
                item,
                items_left,
                items_processed,
            })
            .await?;
        return Ok(());
    };

    let userid = if let Some(user) = msg.from() {
        user.id.0
    } else {
        0
    };
    let keyboard = InlineKeyboardMarkup::default().append_row(
        accounts
            .into_iter()
            .filter(|&l| l.starts_with("Expenses:"))
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
    dialogue: QIFDialogue,
    msg: Message,
    (filename, item, category, mut items_left, mut items_processed): (
        String,
        String,
        String,
        Vec<String>,
        HashMap<String, String>,
    ), // Available from `State::SubCategory`.
) -> HandlerResult {
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

fn nofilter(line: &str) -> &str {
    line
}

async fn handle_qif_ready(
    bot: Bot,
    dialogue: QIFDialogue,
    msg: Message,
    (filename, item_categories): (String, HashMap<String, String>), // Available from `State::Ready`.
) -> HandlerResult {
    let mut user = User::new(msg.chat.id.0, &None);

    let acc = Account::new()
        .name("Reiffeisen")
        .account_type(AccountType::Bank)
        .build();

    let cat = &|item: &str, stats: &mut categories::CatStats, acc: &[String]| -> String {
        item_categories
            .get(&format!("{}", &item))
            .unwrap()
            .to_owned()
    };

    let t = convert(&filename, "testmemo", &mut user, &acc, nofilter, cat).unwrap();
    let qif = InputFile::memory(format!("{}{}", acc, t).into_bytes());
    bot.send_message(msg.chat.id, "QIF is ready.").await?;
    bot.send_document(msg.chat.id, qif).await?;
    dialogue.update(State::Idle).await?;
    Ok(())
}

async fn callback_handler(q: CallbackQuery, bot: Bot, dialogue: QIFDialogue) -> HandlerResult {
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
                        mut items_left,
                        mut items_processed,
                    } = data
                    {
                        log::info!("SubCategory match!");
                        bot.send_message(
                            chat.id,
                            format!("Item {} is ready for caterogy {}", item, version),
                        )
                        .await?;
                        items_processed.insert(item, version);
                        if let Some(newitem) = items_left.pop() {
                            bot.send_message(
                                chat.id,
                                format!("Input category to search for {}", newitem),
                            )
                            .await?;
                            dialogue
                                .update(State::CategorySelect {
                                    filename,
                                    item: newitem,
                                    items_left,
                                    items_processed,
                                })
                                .await?;
                        } else {
                            bot.send_message(chat.id, format!("This was the last item!"))
                                .await?;
                            for (key, value) in &items_processed {
                                bot.send_message(chat.id, format!("{}: {}", key, value))
                                    .await?;
                            }
                            dialogue
                                .update(State::Ready {
                                    filename: filename,
                                    item_categories: items_processed,
                                })
                                .await?;
                        }
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

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .branch(
                    dptree::entry()
                        // Filter commands: the next handlers will receive a parsed `Command`.
                        .filter_command::<Command>()
                        // If a command parsing fails, this handler will not be executed.
                        .endpoint(command_handler),
                )
                .branch(dptree::case![State::Idle].endpoint(handle_idle))
                // No idea about `{filename, }`, but otherwise thread "'tokio-runtime-worker' panicked at '(alloc::string::String,) was requested, but not provided."
                .branch(dptree::case![State::NewJson { filename }].endpoint(handle_json))
                .branch(
                    dptree::case![State::CategorySelect {
                        filename,
                        item,
                        items_left,
                        items_processed,
                    }]
                    .endpoint(handle_category),
                )
                .branch(
                    dptree::case![State::SubCategorySelect {
                        filename,
                        item,
                        category,
                        items_left,
                        items_processed,
                    }]
                    .endpoint(handle_subcategory),
                )
                .branch(
                    dptree::case![State::Ready {
                        filename,
                        item_categories
                    }]
                    .endpoint(handle_qif_ready),
                ),
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
