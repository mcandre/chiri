use std::fmt;
use std::process;

/// ChiriError models unsuccessful program states.
#[derive(Debug)]
pub enum ChiriError {
    IOError(String),
    TOMLParseError(String),
}

impl fmt::Display for ChiriError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChiriError::IOError(e) => write!(f, "{e}"),
            ChiriError::TOMLParseError(e) => write!(f, "{e}"),
        }
    }
}

impl die::PrintExit for ChiriError {
    fn print_exit(&self) -> ! {
        eprintln!("{}", self);
        process::exit(die::DEFAULT_EXIT_CODE);
    }
}
