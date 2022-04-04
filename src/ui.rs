use rustyline::completion::Completer;
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::line_buffer::LineBuffer;
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::{Cmd, CompletionType, Config, Context, EditMode, Editor, KeyEvent};
use rustyline_derive::Helper;
use std::borrow::Cow::{self, Borrowed, Owned};
#[cfg(feature = "tv")]
use std::ffi::CString;
use std::io::{stdout, Write};
#[cfg(feature = "tv")]
use std::os::raw::c_char;
use std::process::exit;

#[cfg(feature = "tv")]
extern "C" {
    fn ui_main(line: *const c_char);
}

#[cfg(feature = "tv")]
#[cfg_attr(tarpaulin, ignore)]
pub fn run_tv() {
    let line = CString::new("I'm calling TV!").expect("Failed to create string");
    unsafe {
        ui_main(line.as_ptr());
    }
    println!("Hello, world!");
}

struct CatCompleter<'a> {
    completions: &'a [&'a String],
}

impl Completer for CatCompleter<'_> {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let results: Vec<String> = self
            .completions
            .iter()
            .filter(|comp| comp.contains(line))
            .map(|s| s.to_string())
            .collect();

        Ok((0, results))
    }

    fn update(&self, _line: &mut LineBuffer, _start: usize, _elected: &str) {}
}

impl<'a> CatCompleter<'a> {
    pub fn new(completions: &'a [&'a String]) -> Self {
        log::debug!("Completions: {:#?}", completions);
        Self { completions }
    }
}

#[derive(Helper)]
struct CatHelper<'a> {
    completer: CatCompleter<'a>,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Completer for CatHelper<'_> {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for CatHelper<'_> {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for CatHelper<'_> {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Validator for CatHelper<'_> {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

pub fn input_category(item: &str, cat: &str, cats: &[&String]) -> String {
    let _ = stdout().flush();

    let config = Config::builder()
        .history_ignore_space(true)
        .check_cursor_position(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();
    let h = CatHelper {
        completer: CatCompleter::new(cats),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };
    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
    rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);
    if rl.load_history("history.txt").is_err() {
        log::debug!("No previous history.");
    }
    let mut result = String::new();
    let p = format!("{} ({})> ", item, cat);
    rl.helper_mut().expect("No helper").colored_prompt =
        format!("\x1b[1;33m{} \x1b[1;32m({})\x1b[0m\x1b[1;37m> ", item, cat);
    let readline = rl.readline(&p);
    match readline {
        Ok(line) => {
            rl.add_history_entry(line.as_str());
            result = line;
        }
        Err(ReadlineError::Interrupted) => {
            println!("Interrupted");
            exit(1);
        }
        Err(ReadlineError::Eof) => {
            println!("Encountered Eof");
        }
        Err(err) => {
            println!("Error: {:?}", err);
        }
    }
    print!("\x1b[1;0m");
    String::from(result.trim_end_matches('\n'))
}
