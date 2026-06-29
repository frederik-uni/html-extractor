mod args;
mod diagnostic;
mod input;
mod run;

use std::{io, io::Read, io::Write, process::ExitCode};

use clap::Parser;
use is_terminal::IsTerminal;

use crate::{args::Args, input::StdinInput, run::CliError};

#[tokio::main]
async fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => error.exit(),
    };
    let stdin = match read_stdin() {
        Ok(stdin) => stdin,
        Err(error) => return report_error(error),
    };
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    match run::run_with_io(args, stdin, &mut stdout, &mut stderr).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let code = error.exit_code();
            let _ = writeln!(stderr, "{error}");
            ExitCode::from(code)
        }
    }
}

fn read_stdin() -> Result<StdinInput, CliError> {
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        Ok(StdinInput::Terminal)
    } else {
        let mut source = String::new();
        stdin.read_to_string(&mut source).map_err(CliError::stdin)?;
        Ok(StdinInput::Redirected(source))
    }
}

fn report_error(error: CliError) -> ExitCode {
    let code = error.exit_code();
    eprintln!("{error}");
    ExitCode::from(code)
}
