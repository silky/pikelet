//! The REPL (Read-Eval-Print-Loop)

use failure::Error;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use codespan::{CodeMap, FileMap, FileName};
use codespan_reporting;
use std::path::PathBuf;
use term_size;

use semantics;
use syntax::parse;

/// Options for the `repl` subcommand
#[derive(Debug, StructOpt)]
pub struct Opts {
    /// The prompt to display before expressions
    #[structopt(long = "prompt", default_value = "Pikelet> ")]
    pub prompt: String,

    /// The history file to record previous commands to (blank to disable)
    #[structopt(long = "history-file", parse(from_os_str), default_value = "repl-history")]
    pub history_file: Option<PathBuf>,

    /// Files to preload into the REPL
    #[structopt(name = "FILE", parse(from_os_str))]
    pub files: Vec<PathBuf>,
}

const LOGO_TEXT: &[&str] = &[
    r"    ____  _ __        __     __     ",
    r"   / __ \(_) /_____  / /__  / /_    ",
    r"  / /_/ / / //_/ _ \/ / _ \/ __/    ",
    r" / ____/ / ,< /  __/ /  __/ /_      ",
    r"/_/   /_/_/|_|\___/_/\___/\__/      ",
    r"",
];

const HELP_TEXT: &[&str] = &[
    "",
    "Command       Arguments   Purpose",
    "",
    "<expr>                    evaluate a term",
    ":? :h :help               display this help text",
    ":q :quit                  quit the repl",
    ":t :type      <expr>      infer the type of an expression",
    "",
];

/// Run the `repl` subcommand with the given options
pub fn run(opts: Opts) -> Result<(), Error> {
    // TODO: Load files

    let mut rl = Editor::<()>::new();
    let mut codemap = CodeMap::new();

    if let Some(ref history_file) = opts.history_file {
        rl.load_history(&history_file)?;
    }

    for (i, line) in LOGO_TEXT.iter().enumerate() {
        match i {
            2 => println!("{}Version {}", line, env!("CARGO_PKG_VERSION")),
            3 => println!("{}{}", line, env!("CARGO_PKG_HOMEPAGE")),
            4 => println!("{}:? for help", line),
            _ => println!("{}", line),
        }
    }

    // TODO: Load files

    loop {
        match rl.readline(&opts.prompt) {
            Ok(line) => {
                if let Some(_) = opts.history_file {
                    rl.add_history_entry(&line);
                }

                let filename = FileName::virtual_("repl");
                match eval_print(&codemap.add_filemap(filename, line)) {
                    Ok(ControlFlow::Continue) => {},
                    Ok(ControlFlow::Break) => break,
                    Err(EvalPrintError::Parse(errs)) => for err in errs {
                        codespan_reporting::emit(&codemap, &err.to_diagnostic());
                    },
                    Err(EvalPrintError::Type(err)) => {
                        codespan_reporting::emit(&codemap, &err.to_diagnostic());
                    },
                }
            },
            Err(err) => match err {
                ReadlineError::Interrupted => println!("Interrupt"),
                ReadlineError::Eof => break,
                err => {
                    println!("readline error: {:?}", err);
                    break;
                },
            },
        }
    }

    if let Some(ref history_file) = opts.history_file {
        rl.save_history(history_file)?;
    }

    println!("Bye bye");

    Ok(())
}

fn eval_print(filemap: &FileMap) -> Result<ControlFlow, EvalPrintError> {
    use std::usize;

    use syntax::concrete::ReplCommand;
    use syntax::core::Context;
    use syntax::pretty::{self, ToDoc};
    use syntax::translation::ToCore;

    fn term_width() -> Option<usize> {
        term_size::dimensions().map(|(width, _)| width)
    }

    let (repl_command, parse_errors) = parse::repl_command(filemap);
    if !parse_errors.is_empty() {
        return Err(EvalPrintError::Parse(parse_errors));
    }

    match repl_command {
        ReplCommand::Help => for line in HELP_TEXT {
            println!("{}", line);
        },

        ReplCommand::Eval(parse_term) => {
            let term = parse_term.to_core();
            let context = Context::new();
            let (_, inferred) = semantics::infer(&context, &term)?;
            let evaluated = semantics::normalize(&context, &term)?;
            let doc = pretty::pretty_ann(pretty::Options::default(), &evaluated, &inferred);

            println!("{}", doc.pretty(term_width().unwrap_or(usize::MAX)));
        },
        ReplCommand::TypeOf(parse_term) => {
            let term = parse_term.to_core();
            let context = Context::new();
            let (_, inferred) = semantics::infer(&context, &term)?;
            let doc = inferred.to_doc(pretty::Options::default());

            println!("{}", doc.pretty(term_width().unwrap_or(usize::MAX)));
        },

        ReplCommand::NoOp | ReplCommand::Error(_) => {},
        ReplCommand::Quit => return Ok(ControlFlow::Break),
    }

    Ok(ControlFlow::Continue)
}

#[derive(Copy, Clone)]
enum ControlFlow {
    Break,
    Continue,
}

enum EvalPrintError {
    Parse(Vec<parse::ParseError>),
    Type(semantics::TypeError),
}

impl From<parse::ParseError> for EvalPrintError {
    fn from(src: parse::ParseError) -> EvalPrintError {
        EvalPrintError::Parse(vec![src])
    }
}

impl From<Vec<parse::ParseError>> for EvalPrintError {
    fn from(src: Vec<parse::ParseError>) -> EvalPrintError {
        EvalPrintError::Parse(src)
    }
}

impl From<semantics::TypeError> for EvalPrintError {
    fn from(src: semantics::TypeError) -> EvalPrintError {
        EvalPrintError::Type(src)
    }
}

impl From<semantics::InternalError> for EvalPrintError {
    fn from(src: semantics::InternalError) -> EvalPrintError {
        EvalPrintError::Type(src.into())
    }
}
