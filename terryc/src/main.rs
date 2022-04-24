#![feature(let_else)]
use std::env::args_os;
use std::path::PathBuf;
use std::{fs, io};

use clap::Command;
use terryc_base::{Providers, Context};
//use terry::interpret::Interpreter;

/// Simple program to greet a person
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    file: PathBuf,

    #[clap(long)]
    use_ascii: bool,
}

fn main() -> io::Result<()> {
    let m: Args = clap::Parser::parse();

    let mut providers = Providers::default();
    terryc_lex::provide(&mut providers);

    terryc_base::GlobalCtxt::create_and_then(terryc_base::Options {
        path: m.file,
        use_ascii: m.use_ascii,
    }, |mut gcx| {
        gcx.set_providers(terryc_base::leak(providers));
        gcx
    });

    /*let s = fs::read_to_string(&m.file)?;
        let lexer = Lexer::new(&s);
        let Ok(tokens) = lexer.scan_tokens() else { std::process::exit(1) };
        let mut parser = Parser::new(&s, &tokens);
        let Ok(ast) = parser.parse_stmts() else { std::process::exit(1) };
        println!("{ast:#?}");
    */
    Ok(())
}
