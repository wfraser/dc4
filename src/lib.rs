//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2022 by William R. Fraser
//

#![deny(rust_2018_idioms)]

mod big_real;
mod dcregisters;
pub mod parser;
pub mod reader_parser;
mod state;

use parser::Action;
use state::Dc4State;
use std::io::{BufRead, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Flavor {
    Gnu,
    Bsd,
    Gavin,
}

/// Desk Calculator 4
pub struct Dc4 {
    state: Dc4State,
}

impl Dc4 {
    /// Make a new DC4 instance with the given name.
    pub fn new(program_name: String, flavor: Flavor) -> Self {
        Self { state: Dc4State::new(program_name, flavor) }
    }

    /// Run a program from a stream of bytes.
    ///
    /// This consumes the entire stream. Errors do not stop the program; they are written to
    /// output, but execution continues.
    pub fn stream(&mut self, r: &mut impl BufRead, w: &mut impl Write) -> DcResult
    {
        let mut actions = reader_parser::ReaderParser::new(r);
        actions.set_flavor(self.state.flavor);
        // There's no safe way to stop mid-stream on an error, because ReaderParser may have read
        // the source stream past the action that caused it, and so returning from here could lose
        // data from the source stream. So you can't really make a `try_stream()` that doesn't do
        // this.
        loop {
            match self.actions(&mut actions, w) {
                Err(e) => self.state.error(w, format_args!("{e}")),
                Ok(result) => return result,
            }
        }
    }

    /// Run a given program text as if it was a macro.
    ///
    /// Errors do not stop the program; they are written to output, but execution continues.
    pub fn text(&mut self, text: impl Into<Vec<u8>>, w: &mut impl Write) -> DcResult {
        self.state.run_macro(text.into(), w)
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

#[derive(Clone, Debug)]
pub enum DcValue {
    Str(Vec<u8>),
    Num(big_real::BigReal)
}

#[derive(Debug)]
pub enum DcResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Macro(Vec<u8>),
}

#[derive(Debug)]
pub enum DcError {
    ArrayIndexInvalid,
    DivideByZero,
    InputError(std::io::Error),
    InputRadixInvalid,
    NegativeExponent,
    NonNumericValue,
    OutputRadixInvalid,
    QuitInvalid,
    QuitTooBig,
    RegisterEmpty(u8),
    RemainderByZero,
    ScaleInvalid,
    ScaleTooBig,
    ShellUnsupported,
    SqrtNegative,
    SqrtNonNumeric,
    StackEmpty,
    StackRegisterEmpty(u8),
    UnexpectedNumberChar(u8),
    Unimplemented(u8),
}

impl std::fmt::Display for DcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DcError::*;
        // error messages should match those from GNU dc as much as possible
        match self {
            ArrayIndexInvalid => f.write_str("array index must be a nonnegative integer"),
            DivideByZero => f.write_str("divide by zero"),
            InputError(e) => write!(f, "error reading input: {e}"),
            InputRadixInvalid => f.write_str("input base must be a number between 2 and 16 (inclusive)"),
            NegativeExponent => f.write_str("negative exponent"),
            NonNumericValue => f.write_str("non-numeric value"),
            OutputRadixInvalid => f.write_str("output base must be a number between 2 and 16 (inclusive)"),
            QuitInvalid => f.write_str("Q command requires a number >= 1"),
            QuitTooBig => f.write_str("quit levels out of range (must fit into 32 bits)"),
            RegisterEmpty(r) => write!(f, "register '{}' (0{r:o}) is empty", *r as char),
            RemainderByZero => f.write_str("remainder by zero"),
            ScaleInvalid => f.write_str("scale must be a nonnegative integer"),
            ScaleTooBig => f.write_str("scale must fit into 32 bits"),
            ShellUnsupported => f.write_str("running shell commands is not supported"),
            SqrtNegative => f.write_str("square root of negative number"),
            SqrtNonNumeric => f.write_str("square root of nonnumeric attempted"),
            StackEmpty => f.write_str("stack empty"),
            StackRegisterEmpty(r) => write!(f, "stack register '{}' (0{r:o}) is empty", *r as char),
            UnexpectedNumberChar(c) => write!(f, "unexpected character in number: {:?}", *c as char),
            Unimplemented(c) => write!(f, "{:?} (0{c:o}) unimplemented", *c as char),
        }
    }
}

impl std::error::Error for DcError {}
