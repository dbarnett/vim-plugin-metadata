//! # vim-plugin-metadata
//!
//! A library to parse and analyze your vim plugins.
//!
//! The main use case is to instantiate a [VimParser], configure it, and point
//! it to a plugin dir or file to parse.

mod data;
mod parser;

pub use crate::data::{VimModule, VimNode, VimPlugin};
pub use crate::parser::VimParser;

use core::fmt;
use std::{error, io};

#[derive(Debug)]
pub enum Error {
    UnknownError(Box<dyn error::Error>),
    GrammarError(tree_sitter::LanguageError),
    ParsingFailure,
    IOError(io::Error),
}

impl From<tree_sitter::LanguageError> for Error {
    fn from(e: tree_sitter::LanguageError) -> Self {
        Self::GrammarError(e)
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        if err.io_error().is_some() {
            err.into_io_error().unwrap().into()
        } else {
            Self::UnknownError(err.into())
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IOError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownError(err) => write!(f, "Unknown error: {err}"),
            Self::GrammarError(err) => write!(f, "Error loading grammar: {err}"),
            Self::ParsingFailure => {
                write!(f, "General failure from tree-sitter while parsing syntax")
            }
            Self::IOError(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl error::Error for Error {}

type Result<T> = core::result::Result<T, Error>;
