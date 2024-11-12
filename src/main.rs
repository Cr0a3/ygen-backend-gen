
pub mod ast;
use std::io::Read;

use crate::ast::*;

pub mod gen;


use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// the input file
    input: String,
}

fn main() {
    let args = Args::parse();

    let infile = args.input;

    let mut file = match std::fs::File::open(infile) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(-1);
        },
    };

    let mut input = String::new();

    match file.read_to_string(&mut input) {
        Ok(_) => {},
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(-1);
        },
    };

    let patterns = ast::parse(&input);

    let emiter = gen::CodeEmitter {
        patterns: patterns
    };

    println!("{}", emiter.gen(AstTarget::X86));
}
