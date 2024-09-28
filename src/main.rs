use kast_ast as ast;
use kast_util::*;
use std::{
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
};

mod cli;
mod compiler;
mod id;
mod inference;
mod interpreter;
mod ir;
mod ty;
mod value;

use id::*;
use ir::*;
use kast_ast::{Ast, Token};
use ty::*;
use value::*;

struct Kast {
    compiler: compiler::State,
    interpreter: interpreter::State,
}

impl Kast {
    fn new() -> Self {
        Self {
            compiler: compiler::State::new(),
            interpreter: interpreter::State::new(),
        }
    }
}

fn std_path() -> PathBuf {
    match std::env::var_os("KAST_STD") {
        Some(path) => path.into(),
        None => match option_env!("CARGO_MANIFEST_DIR") {
            Some(path) => Path::new(path).join("std"),
            None => panic!("kast standard library not found"),
        },
    }
}

fn run_repl(mut handler: impl FnMut(String) -> eyre::Result<()>) -> eyre::Result<()> {
    let mut rustyline = rustyline::DefaultEditor::with_config(
        rustyline::Config::builder().auto_add_history(true).build(),
    )?;
    let is_tty = std::io::stdin().is_terminal();
    tracing::debug!("is tty: {is_tty:?}");
    loop {
        let s = match is_tty {
            true => match rustyline.readline("> ") {
                Ok(line) => line,
                Err(rustyline::error::ReadlineError::Eof) => break,
                Err(e) => return Err(e.into()),
            },
            false => {
                let mut contents = String::new();
                std::io::stdin()
                    .lock()
                    .read_to_string(&mut contents)
                    .unwrap();
                contents
            }
        };
        handler(s)?;
        if !is_tty {
            break;
        }
    }
    Ok(())
}

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    let cli_args = cli::parse();
    match cli_args.command {
        cli::Command::ParseAst => {
            let syntax = ast::read_syntax(SourceFile {
                contents: std::fs::read_to_string(std_path().join("syntax.ks")).unwrap(),
                filename: "std/syntax.ks".into(),
            })?;
            tracing::trace!("{syntax:#?}");
            run_repl(|contents| {
                let source = SourceFile {
                    contents,
                    filename: "<stdin>".into(),
                };
                let ast = ast::parse(&syntax, source)?;
                match ast {
                    None => println!("<nothing>"),
                    Some(ast) => println!("{ast:#}"),
                }
                Ok(())
            })?;
        }
        cli::Command::Repl => {
            let syntax = ast::read_syntax(SourceFile {
                contents: std::fs::read_to_string(std_path().join("syntax.ks")).unwrap(),
                filename: "std/syntax.ks".into(),
            })?;
            let mut kast = Kast::new();
            run_repl(|contents| {
                let source = SourceFile {
                    contents,
                    filename: "<stdin>".into(),
                };
                let ast = ast::parse(&syntax, source)?;
                let Some(ast) = ast else {
                    // empty line
                    return Ok(());
                };
                let value = kast.eval_ast(ast);
                println!("{} :: {}", value, value.ty());
                Ok(())
            })?;
        }
    }

    Ok(())
}
