//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2019 by William R. Fraser
//

extern crate num;

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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match *self {
            DCError::Message(ref msg) => msg,
            DCError::StaticMessage(msg) => msg,
        };
        f.write_str(msg)
    }
}

impl Into<DCError> for String {
    fn into(self) -> DCError {
        DCError::Message(self)
    }
}

impl Into<DCError> for &'static str {
    fn into(self) -> DCError {
        DCError::StaticMessage(self)
    }
}
