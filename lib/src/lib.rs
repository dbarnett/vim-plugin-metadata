mod data;
mod parser;

pub use crate::data::VimNode;
pub use crate::parser::VimParser;

use core::fmt;
use std::error;
use tree_sitter::LanguageError;

#[derive(Debug)]
pub enum Error {
    GrammarError(LanguageError),
    ParsingFailure,
}

impl From<LanguageError> for Error {
    fn from(e: LanguageError) -> Self {
        Self::GrammarError(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GrammarError(e) => write!(f, "Error loading grammar: {e}"),
            Self::ParsingFailure => {
                write!(f, "General failure from tree-sitter while parsing syntax")
            }
        }
    }
}

impl error::Error for Error {}

type Result<T> = core::result::Result<T, Error>;
