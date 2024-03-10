use crate::categories;
use crate::convert::{auto_cat_items, convert};
use qif_generator::account::{Account, AccountType};

#[cfg(feature = "monitoring")]
use crate::monitoring;
use crate::tgusermanager::user_manager;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::tgusermanager::TgManagerCommand;
use derive_more::From;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardButtonKind, InlineKeyboardMarkup, InputFile, MediaKind,
    MessageKind, ReplyMarkup,
};
use teloxide::{
    dispatching::dialogue::InMemStorage, net::Download, prelude::*, types::File as TgFile,
    utils::command::BotCommands, DownloadError, RequestError,
};
use thiserror::Error;
use tokio::fs::File;
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

/// Possible error while receiving a file
#[cfg(feature = "telegram")]
#[derive(Debug, Error, From)]
enum UserManagerError {
    /// Manager didn't respond
    #[error("Couldn't request user: {0}")]
    Request(String),
}

use tokio::sync::mpsc::Sender;

struct ManagerHandle<T> {
    tx: Sender<T>,
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

    #[command(description = "Create new account")]
    NewAccount { account: String },

    #[command(description = "List accounts")]
    Accounts,
}

async fn command_handler(
    bot: Bot,
    dialogue: QIFDialogue,
    _me: teloxide::types::Me,
    msg: Message,
    cmd: Command,
    manager_handle: Arc<ManagerHandle<TgManagerCommand>>,
) -> HandlerResult {
    let tx = &manager_handle.tx;

    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Start => {
            bot.send_message(msg.chat.id, "Starting".to_string())
                .await?
        }
        Command::Delete => {
            bot.send_message(msg.chat.id, "Deleting".to_string())
                .await?
        }

        Command::Request => {
            bot.send_message(msg.chat.id, "Requesting".to_string())
                .await?
        }
        Command::Cancel => {
            log::info!("Reset requested");
            dialogue.update(State::Idle).await?;
            bot.send_message(msg.chat.id, "Dialogue state reset".to_string())
                .await?
        }
        Command::NewAccount { account } => {
            let acc_to_add = account.trim();

            if acc_to_add.is_empty() {
                log::warn!("/newaccount executed without account name");
                bot.send_message(msg.chat.id, "No account name provided".to_string())
                    .await?
            } else {
                let (response_tx, response_rx) = oneshot::channel();

                tx.send(TgManagerCommand::Get {
                    user_id: msg.chat.id.0,
                    reply_to: response_tx,
                })
                .await?;

                if let Ok(mut user) = response_rx.await {
                    user.new_account(String::from(acc_to_add));
                    bot.send_message(msg.chat.id, "Account added".to_string())
                        .await?
                } else {
                    log::error!("Request for unknown userid {}", msg.chat.id.0);
                    bot.send_message(msg.chat.id, "Can't find the requested user".to_string())
                        .await?
                }
            }
        }
        Command::Accounts => {
            let (response_tx, response_rx) = oneshot::channel();

            tx.send(TgManagerCommand::Get {
                user_id: msg.chat.id.0,
                reply_to: response_tx,
            })
            .await?;

            if let Ok(user) = response_rx.await {
                let list = |expense_set: &HashSet<String>| {
                    let mut sorted_expenses: Vec<String> = expense_set
                        .iter()
                        .filter(|s| s.starts_with("Expenses:"))
                        .map(|s| s.trim_start_matches("Expenses:").to_owned())
                        .collect();

                    sorted_expenses.sort();

                    sorted_expenses
                        .into_iter()
                        .map(|s| s + "\n")
                        .collect::<String>()
                };

                bot.send_message(
                    msg.chat.id,
                    format!("Expense accounts:\n\n{}", list(&user.accounts)),
                )
                .await?
            } else {
                log::error!("Request for unknown userid {}", msg.chat.id.0);
                bot.send_message(msg.chat.id, "Can't find the requested user".to_string())
                    .await?
            }
        }
    };

    Ok(())
}

async fn download_file(downloader: &Bot, file_id: &str) -> Result<String, FileReceiveError> {
    let TgFile { path, .. } = downloader.get_file(file_id).send().await?;
    log::info!("Attempt to download file");
    let filepath = format!("/tmp/{}", file_id);
    log::info!("Path: {}", filepath);
    let mut file = File::create(&filepath).await?;
    downloader.download_file(&path, &mut file).await?;
    Ok(filepath)
}

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
                items_left: _,
                items_processed: _,
            } => {
                write!(f, "Category: {}, {}", filename, item)
            }
            State::SubCategorySelect {
                filename,
                item,
                category,
                items_left: _,
                items_processed: _,
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
    log::debug!("Idle state");
    bot.send_message(msg.chat.id, "Upload your file").await?;
    dialogue
        .update(State::NewJson {
            filename: "test".to_string(),
        })
        .await?;
    Ok(())
}

/*
/// List all the items' categories when the all the ticket is processed
fn format_categories(catitems: &HashMap<String, String>) -> String {
    catitems
        .iter()
        .enumerate()
        .fold(String::new(), |mut acc, (index, (item, category))| {
            use std::fmt::Write;
            writeln!(
                &mut acc,
                "{}. [{}: {}](/edit_{})",
                index + 1,
                item,
                category,
                item
            )
            .unwrap();
            acc
        })
}
 */

fn create_categories_keyboard(catitems: &HashMap<String, String>) -> InlineKeyboardMarkup {
    let mut keyboard = InlineKeyboardMarkup::default(); // Use default to initialize

    for (index, (item, category)) in catitems.iter().enumerate() {
        let parts: Vec<&str> = category.split(':').collect();
        let shortened_category = if parts.len() > 1 {
            parts[..parts.len() - 1]
                .iter()
                .map(|&part| {
                    part.chars()
                        .next()
                        .unwrap_or_default()
                        .to_uppercase()
                        .collect::<String>()
                })
                .chain(std::iter::once(parts.last().unwrap().to_string()))
                .collect::<Vec<String>>()
                .join(":")
        } else {
            category.clone()
        };

        let button_text = format!("{}: {}", item, shortened_category);
        // Using only the index as callback data to avoid exceeding the maximum length
        let callback_data = format!("edit_{}", index);
        log::info!("Text: '{}'  Data: '{}'", button_text, callback_data);

        let button = InlineKeyboardButton::callback(button_text, callback_data);
        keyboard = keyboard.append_row(vec![button]);
    }
    keyboard
}

/// Fuzzy matcher for "A:B" to "ACategory:BSubCategory"
fn filter_categories<'a, I>(categories: I, input: &str) -> Vec<&'a String>
where
    I: Iterator<Item = &'a String>,
{
    let input_parts: Vec<&str> = input.split(':').collect();
    if input_parts.is_empty() || input_parts.iter().any(|part| part.is_empty()) {
        return Vec::new();
    }

    categories
        .filter(|category| {
            let cat = category.to_lowercase();
            let cat_parts: Vec<&str> = cat.split(':').collect();
            cat_parts.windows(input_parts.len()).any(|window| {
                input_parts
                    .iter()
                    .zip(window.iter())
                    .all(|(&inp, &win)| win.starts_with(inp))
            })
        })
        .collect()
}

async fn handle_json(
    bot: Bot,
    dialogue: QIFDialogue,
    msg: Message,
    filename: String, // Available from `State::Idle`.
    manager_handle: Arc<ManagerHandle<TgManagerCommand>>,
) -> HandlerResult {
    log::debug!("JSON state");
    log::info!("File {}", &filename);
    #[cfg(feature = "monitoring")]
    monitoring::INCOMING_REQUESTS.inc();

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
    }

    if let Ok(newfile) = download_file(&bot, &file_id).await {
        log::info!("Active user: {:} File received: {:} ", msg.chat.id, newfile);
        let tx = &manager_handle.tx;
        let (response_tx, response_rx) = oneshot::channel();

        tx.send(TgManagerCommand::Get {
            user_id: msg.chat.id.0,
            reply_to: response_tx,
        })
        .await?;

        let user = response_rx.await.map_err(|_| {
            log::warn!("No response for TgUserManager");
            Box::new(UserManagerError::Request(
                "No response for TgUserManager".to_string(),
            ))
        })?;

        let (cat, mut uncat) = auto_cat_items(&newfile, &user);

        log::debug!("Categorized item list: {:?}", cat);
        log::debug!("Non-categorized item list: {:?}", uncat);

        if uncat.is_empty() {
            log::info!("Automatically categorized");
            bot.send_message(
                msg.chat.id,
                "Items are categorized and categories are updated".to_string(),
            )
            .reply_markup(create_categories_keyboard(&cat))
            .await?;

            bot.send_message(
                msg.chat.id,
                "All the items were categorized automatically\nEnter the memo line".to_string(),
            )
            .await?;
            dialogue
                .update(State::Ready {
                    filename: newfile,
                    item_categories: cat,
                })
                .await?;
        } else if let Some(item) = uncat.pop() {
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
                    items_left: uncat,
                    items_processed: cat,
                })
                .await?;
        } else {
            log::error!("Can't pop from non-empty list");
        }
    } else {
        log::warn!("Malformed json or categorization problem");
        bot.send_message(msg.chat.id, "Can't parse the provided file".to_string())
            .await?;
        dialogue
            .update(State::NewJson {
                filename: String::new(),
            })
            .await?;
    }

    Ok(())
}

async fn handle_category(
    bot: Bot,
    dialogue: QIFDialogue,
    msg: Message,
    manager_handle: Arc<ManagerHandle<TgManagerCommand>>,
    (filename, item, items_left, items_processed): (
        String,
        String,
        Vec<String>,
        HashMap<String, String>,
    ), // Available from `State::NewJson`.
) -> HandlerResult {
    log::debug!("Category state");
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

    let tx = &manager_handle.tx;
    let (response_tx, response_rx) = oneshot::channel();

    tx.send(TgManagerCommand::Get {
        user_id: msg.chat.id.0,
        reply_to: response_tx,
    })
    .await?;

    let user = response_rx.await.map_err(|_| {
        log::warn!("No response for TgUserManager");
        Box::new(UserManagerError::Request(
            "No response for TgUserManager".to_string(),
        ))
    })?;

    let mut accounts = if version.contains(':') {
        filter_categories(user.accounts.iter(), &version.to_lowercase())
    } else {
        user.accounts
            .iter()
            .filter(|&e| {
                e.starts_with("Expenses:") && e.to_lowercase().contains(&version.to_lowercase())
            })
            .collect::<Vec<_>>()
    };

    accounts.sort_unstable();

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
    log::debug!("SubCategory state");
    match msg.text() {
        Some(_subcategory) => {
            bot.send_message(msg.chat.id, "Item ready").await?;
            items_processed.insert(item, category);
            if !items_left.is_empty() {
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
                    log::error!("Can't pop next item :(");
                    bot.send_message(msg.chat.id, "Internal error happened".to_string())
                        .await?;
                    dialogue
                        .update(State::NewJson {
                            filename: String::new(),
                        })
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

async fn handle_qif_ready(
    bot: Bot,
    dialogue: QIFDialogue,
    msg: Message,
    manager_handle: Arc<ManagerHandle<TgManagerCommand>>,
    (filename, item_categories): (String, HashMap<String, String>), // Available from `State::Ready`.
) -> HandlerResult {
    log::debug!("QIF Ready state");
    let tx = &manager_handle.tx;
    let (response_tx, response_rx) = oneshot::channel();

    tx.send(TgManagerCommand::Get {
        user_id: msg.chat.id.0,
        reply_to: response_tx,
    })
    .await?;

    let mut user = response_rx.await.map_err(|_| {
        log::warn!("No response for TgUserManager");
        Box::new(UserManagerError::Request(
            "No response for TgUserManager".to_string(),
        ))
    })?;

    let memo: &str = msg.text().unwrap_or("purchase");

    let acc = Account::new()
        .name("Reiffeisen")
        .account_type(AccountType::Bank)
        .build();

    // TODO: Check if we need to assign categories by default
    for (i, c) in &item_categories {
        if c.is_empty() {
            log::error!("QIF is ready with no category for item {:}", i);
            bot.send_message(msg.chat.id, "Internal error happened".to_string())
                .await?;
            dialogue
                .update(State::NewJson {
                    filename: String::new(),
                })
                .await?;
            return Ok(());
        }

        categories::assign_category(i, c, &mut user.catmap);
    }

    let cat = &|item: &str, _stats: &mut categories::CatStats, _acc: &HashSet<String>| -> String {
        item_categories.get(item).unwrap().to_owned()
    };

    let filter = categories::LineFilter::new()
        .numfilter()
        .perekrestok_filter()
        .trim_units_from_end()
        .build();

    let t = convert(&filename, memo, &mut user, &acc, filter, cat).unwrap();
    let qif = InputFile::memory(format!("{}{}", acc, t).into_bytes());
    bot.send_message(msg.chat.id, "QIF is ready.").await?;
    bot.send_document(msg.chat.id, qif).await?;

    dialogue
        .update(State::NewJson {
            filename: String::new(),
        })
        .await?;

    Ok(())
}

async fn callback_handler(q: CallbackQuery, bot: Bot, dialogue: QIFDialogue) -> HandlerResult {
    if let Some(version) = q.data {
        if version.starts_with("edit_") {
            let item_id = version.strip_prefix("edit_").unwrap(); // Extract the item ID or number

            // Process the selection, e.g., by updating the dialogue state or responding to the user
            let response_message = format!("Editing item {}", item_id);
            if let Some(chat_id) = q.message.clone().map(|msg| msg.chat.id) {
                bot.send_message(chat_id, response_message).await?;

                let state = dialogue.get().await?;
                if let Some(data) = state {
                    log::info!("State: {}", data);
                    if let State::Ready {
                        filename,
                        item_categories,
                    } = data
                    {
                        let mut item_to_edit = None;
                        let req_item: usize = item_id.parse().unwrap_or_default();
                        for (index, (key, value)) in item_categories.iter().enumerate() {
                            log::debug!("Index: {}, Key: {}, Value: {}", index, key, value);
                            if index == req_item {
                                log::info!("Editing item {}:{}", key, value);
                                item_to_edit = Some(key.clone());
                            }
                        }

                        if let Some(key) = item_to_edit {
                            bot.send_message(
                                chat_id,
                                format!("Input category to search for {}", &key),
                            )
                            .await?;
                            dialogue
                                .update(State::CategorySelect {
                                    filename,
                                    item: key,
                                    items_left: vec![],
                                    items_processed: item_categories,
                                })
                                .await?;
                        } else {
                            log::error!("Attempt to edit non-existent item");
                        }
                    }
                }
            }
        }

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
                        category: _,
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
                            bot.send_message(chat.id, "This was the last item!".to_string())
                                .await?;
                            bot.send_message(
                                chat.id,
                                "Items are categorized and categories are updated".to_string(),
                            )
                            .reply_markup(create_categories_keyboard(&items_processed))
                            .await?;

                            bot.send_message(chat.id, "Enter the memo line".to_string())
                                .await?;
                            dialogue
                                .update(State::Ready {
                                    filename,
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
    #[cfg(feature = "monitoring")]
    let monitoring_handle = tokio::spawn(async move { monitoring::web_main().await });

    log::info!("Starting telegram bot");
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move { user_manager(&mut rx).await });

    let manager_handle = Arc::new(ManagerHandle { tx });

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
        .dependencies(dptree::deps![InMemStorage::<State>::new(), manager_handle])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    drop(manager);
    #[cfg(feature = "monitoring")]
    monitoring_handle.await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_categories_basic_matching() {
        let categories = vec![
            "seg1:seg2:seg3".to_string(),
            "seg1:segX:seg3".to_string(),
            "segA:seg2:segB".to_string(),
        ];

        let filtered = filter_categories(categories.iter(), "seg1:seg2");
        assert_eq!(filtered, vec![&categories[0]]);
        let filtered = filter_categories(categories.iter(), "segx:seg3");
        assert_eq!(filtered, vec![&categories[1]]);
    }

    #[test]
    fn test_filter_categories_partial_match() {
        let categories = vec![
            "seg1:seg2:seg3".to_string(),
            "seg1:seg2X:seg3".to_string(),
            "seg1:seg2".to_string(),
        ];

        let filtered = filter_categories(categories.iter(), "seg1:seg2");
        assert_eq!(
            filtered,
            vec![&categories[0], &categories[1], &categories[2]]
        );
    }

    #[test]
    fn test_filter_categories_empty_input() {
        let categories = vec!["seg1:seg2:seg3".to_string(), "seg4:seg5:seg6".to_string()];

        let filtered = filter_categories(categories.iter(), "");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_categories_no_match() {
        let categories = vec!["seg1:seg2:seg3".to_string(), "seg4:seg5:seg6".to_string()];

        let filtered = filter_categories(categories.iter(), "segX:segY");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_categories_single_segment_input() {
        let categories = vec![
            "seg1:seg2:seg3".to_string(),
            "seg1:seg4:seg5".to_string(),
            "segX:segY:segZ".to_string(),
        ];

        let filtered = filter_categories(categories.iter(), "seg1");
        assert_eq!(filtered, vec![&categories[0], &categories[1]]);
    }
}
