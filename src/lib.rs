//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2019 by William R. Fraser
//

#[macro_use] extern crate log;
extern crate num;
extern crate utf8;

mod big_real;
mod dcregisters;
pub mod parser;
mod state;

pub use state::DC4;

#[derive(Clone, Debug)]
pub enum DCValue {
    Str(String),
    Num(big_real::BigReal)
}

#[derive(Debug)]
pub enum DCResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Recursion(String),
}

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

