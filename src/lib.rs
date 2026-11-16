mod ansi_parser;
mod attr_utils;

pub use ansi_parser::{AnsiParser, AnsiParserBuilder};

#[cfg(test)]
mod tests;
