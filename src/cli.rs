use std::path::PathBuf;

use clap::{value_parser, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, default_value_t = 30_000, value_parser = value_parser!(u32).range(1..))]
    pub array_size: u32,

    #[arg(long, value_enum, default_value_t = EofBehaviour::Ignore)]
    pub eof_behaviour: EofBehaviour,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Run {
        input: PathBuf,
    },
    Compile {
        input: PathBuf,

        #[arg(short)]
        output: Option<PathBuf>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EofBehaviour {
    Ignore,
    Zero,
}
