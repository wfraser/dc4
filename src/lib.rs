//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2021 by William R. Fraser
//

#![deny(rust_2018_idioms)]

mod big_real;
mod dcregisters;
pub mod parser;
mod reader_parser;
mod state;

use parser::Action;
use state::Dc4State;
use std::io::{BufRead, Write};

/// Desk Calculator 4
pub struct Dc4 {
    state: Dc4State,
}

impl Dc4 {
    /// Make a new DC4 instance with the given name.
    pub fn new(program_name: String) -> Self {
        Self { state: Dc4State::new(program_name) }
    }

    /// Run a program from a stream of bytes.
    ///
    /// This consumes the entire stream. Errors do not stop the program; they are written to
    /// output, but execution continues.
    pub fn stream(&mut self, r: &mut impl BufRead, w: &mut impl Write) -> DcResult
    {
        let mut actions = reader_parser::ReaderParser::new(r);
        // There's no safe way to stop mid-stream on an error, because ReaderParser may have read
        // the source stream past the action that caused it, and so returning from here could lose
        // data from the source stream. So you can't really make a `try_stream()` that doesn't do
        // this.
        loop {
            match self.actions(&mut actions, w) {
                Err(e) => self.state.error(w, format_args!("{}", e)),
                Ok(result) => return result,
            }
        }
    }

    /// Run a given program text as if it was a macro.
    ///
    /// Errors do not stop the program; they are written to output, but execution continues.
    pub fn text(&mut self, text: impl AsRef<[u8]>, w: &mut impl Write) -> DcResult {
        let actions = parser::Parser::default()
            .parse(text.as_ref().iter().cloned());
        self.state.run_macro(Rc::new(actions), w)
    }

    /// Run a program from an iterator of actions.
    ///
    /// Stops on the first error encountered.
    pub fn actions(&mut self, actions: impl Iterator<Item = Action>, w: &mut impl Write)
        -> Result<DcResult, DcError>
    {
        for action in actions {
            let mut result = self.state.action(action, w);
            if let Ok(DcResult::Macro(text)) = result {
                result = Ok(self.state.run_macro(text, w));
            }
            match result {
                Ok(DcResult::Continue) => (),
                Ok(DcResult::QuitLevels(_)) => (), // 'Q' mustn't exit the top level
                Ok(other) => return Ok(other),
                Err(e) => return Err(e),
            }
        }
        Ok(DcResult::Continue)
    }

    /// Convenience function for pushing a number onto the stack. Returns Err if the given string
    /// is not a valid number.
    pub fn push_number(&mut self, input: impl AsRef<[u8]>) -> Result<(), DcError> {
        self.state.push_number(input)
    }

    /// Convenience function for pushing a string directly onto the stack (rather than running
    /// Action::StringChar for each byte, followed by Action::PushString).
    pub fn push_string(&mut self, string: impl Into<Vec<u8>>) {
        self.state.push_string(string)
    }

    /// Run a single action.
    ///
    /// Any output gets written to the given writer.
    ///
    /// Errors get returned to the caller and are not written to the writer, but any warnings will
    /// get written as output.
    pub fn action(&mut self, action: Action, w: &mut impl Write) -> Result<DcResult, DcError> {
        self.state.action(action, w)
    }
}

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum DcValue {
    Str(DcString),
    Num(big_real::BigReal)
}

#[derive(Clone, Debug)]
pub struct DcString {
    bytes: Vec<u8>,
    actions: Rc<RefCell<Option<Rc<Vec<parser::Action>>>>>, // lol
}

impl DcString {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
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
        self.bytes
    }
}

#[derive(Debug)]
pub enum DcResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Macro(Rc<Vec<parser::Action>>),
}

#[derive(Debug)]
pub enum DcError {
    Message(String),
    StaticMessage(&'static str),
}

impl std::fmt::Display for DcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match *self {
            DcError::Message(ref msg) => msg,
            DcError::StaticMessage(msg) => msg,
        };
        f.write_str(msg)
    }
}

impl From<String> for DcError {
    fn from(s: String) -> DcError {
        DcError::Message(s)
    }
}

impl From<&'static str> for DcError {
    fn from(s: &'static str) -> DcError {
        DcError::StaticMessage(s)
    }
}
