mod ansi_parser;
mod attr_utils;

pub use ansi_parser::{AnsiError, AnsiErrorKind, AnsiParser, AnsiParserBuilder, OnError};

#[cfg(test)]
mod tests;
