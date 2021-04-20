//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2020 by William R. Fraser
//

#![deny(rust_2018_idioms)]

mod big_real;
mod dcregisters;
pub mod parser;
mod reader_parser;
mod state;

pub use state::DC4;

#[derive(Clone, Debug)]
pub enum DCValue {
    Str(Vec<u8>),
    Num(big_real::BigReal)
}

#[derive(Debug)]
pub enum DCResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Macro(Vec<u8>),
}

#[derive(Debug)]
pub enum DCError {
    Message(String),
    StaticMessage(&'static str),
}

impl std::fmt::Display for DCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match *self {
            DCError::Message(ref msg) => msg,
            DCError::StaticMessage(msg) => msg,
        };
        f.write_str(msg)
    }
}

impl From<String> for DCError {
    fn from(s: String) -> DCError {
        DCError::Message(s)
    }
}

impl From<&'static str> for DCError {
    fn from(s: &'static str) -> DCError {
        DCError::StaticMessage(s)
    }
}
