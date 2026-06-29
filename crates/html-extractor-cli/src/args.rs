use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "html-extractor", subcommand_negates_reqs = true)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
    #[arg(value_name = "FILE", required = true)]
    pub(crate) file: Option<PathBuf>,
    #[arg(long, value_name = "JSON")]
    pub(crate) variables: Option<String>,
    #[arg(long = "variable", value_name = "NAME=VALUE")]
    pub(crate) variable: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Fmt {
        #[arg(short, long)]
        write: bool,
        path: PathBuf,
    },
    Test {
        path: PathBuf,
        #[arg(long)]
        reset_cache: bool,
    },
}
