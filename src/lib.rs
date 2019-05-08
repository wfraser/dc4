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

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum DCValue {
    Str(DCString),
    Num(big_real::BigReal)
}

#[derive(Clone, Debug)]
pub struct DCString {
    bytes: Rc<Vec<u8>>,
    actions: Rc<RefCell<Option<Rc<Vec<parser::Action>>>>>, // lol
}

impl DCString {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Rc::new(bytes),
            actions: Rc::new(RefCell::new(None)),
        }
    }

    pub fn actions(&self) -> Rc<Vec<parser::Action>> {
        if self.actions.borrow().is_none() {
            let actions = parser::Parser::default().parse(self.bytes.iter().cloned());
            *self.actions.borrow_mut() = Some(Rc::new(actions));
        }
        self.actions.borrow().as_ref().unwrap().clone()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        Rc::try_unwrap(self.bytes)
            .unwrap_or_else(|rc| (&*rc).to_owned())
    }
}

#[derive(Debug)]
pub enum DCResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Macro(Rc<Vec<parser::Action>>),
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
