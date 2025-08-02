use std::fs;

pub mod cli;
pub mod compiler;
pub mod interpreter;
pub mod parser;
pub mod program;

use clap::Parser as _;
use parser::Parser;

use crate::{
    cli::Commands::{Compile, Run},
    compiler::Compiler,
    interpreter::Interpreter,
};

fn main() {
    let args = cli::Args::parse();

    let content = match &args.command {
        Run { input } => fs::read_to_string(input),
        Compile { input, output: _ } => fs::read_to_string(input),
    }
    .expect("failed to read input file");

    let mut parser = Parser::new(&content);

    let program = parser.parse().expect("failed to parse program");

    match args.command {
        Run { input: _ } => {
            let mut interpreter = Interpreter::new(args.array_size as usize, args.eof_behaviour);
            interpreter.run(&program);
        }
        Compile { input, output } => {
            let compiler = Compiler::new(args.array_size as usize, args.eof_behaviour);
            compiler.compile(&program, output.unwrap_or(input.with_extension("o")));
        }
    }
}
