//
// sum-numbers :: Accumulate and sum whitespace-delimited numbers from input.
//
// Copyright (c) 2019-2020 by William R. Fraser
//

#![deny(rust_2018_idioms)]

/// This is an example of how DC4 can be used as a library for doing useful numeric operations.
/// The program reads numbers from input, delimited by whitespace, and uses DC4 to add them up as
/// it reads them. When it reaches EOF, it prints the resulting sum. Because it uses DC4, it
/// supports arbitrary precision.

use dc4::DC4;
use dc4::parser::Action;
use std::io::{self, Bytes, Read, Write};

struct Input<R: Read> {
    inner: Bytes<R>,
    buf: Vec<u8>,
    decimal: bool,
}

impl<R: Read> Input<R> {
    pub fn new(reader: R) -> Self {
        Self {
            inner: reader.bytes(),
            buf: vec![],
            decimal: false,
        }
    }
}

impl<R: Read> Iterator for Input<R> {
    type Item = io::Result<Vec<u8>>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let c = match self.inner.next() {
                Some(Ok(c)) => c,
                Some(Err(e)) => {
                    return Some(Err(e));
                }
                None => {
                    if self.buf.is_empty() {
                        return None;
                    } else {
                        self.decimal = false;
                        return Some(Ok(self.buf.split_off(0)));
                    }
                }
            };

            if (c as char).is_whitespace() {
                if !self.buf.is_empty() {
                    self.decimal = false;
                    return Some(Ok(self.buf.split_off(0)));
                }
            } else if c == b'.' {
                if self.decimal {
                    return Some(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid number in input: \"{}{}\"",
                            String::from_utf8_lossy(&self.buf), c as char))));
                } else {
                    self.decimal = true;
                    self.buf.push(c);
                }
            } else if c == b'-' {
                if self.buf.is_empty() {
                    self.buf.push(c);
                } else {
                    return Some(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid number in input: \"{}{}\"",
                            String::from_utf8_lossy(&self.buf), c as char))));
                }
            } else if (b'0' ..= b'9').contains(&c) {
                self.buf.push(c);
            } else {
                return Some(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid character in input: {:?}", c as char))));
            }
        }
    }
}

struct Options {
    iradix: u32,
    oradix: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iradix: 10,
            oradix: 10,
        }
    }
}

impl Options {
    fn parse(mut args: impl Iterator<Item=String>) -> Result<Self, String> {
        let mut opts = Options::default();
        let arg0 = args.next().unwrap();
        while let Some(arg) = args.next() {
            if arg == "-h" || arg == "--help" {
                return Err(format!("usage: {} [-i iradix] [-o oradix]", arg0));
            } else if arg.starts_with("-i") || arg.starts_with("-o") {
                let n = if arg.len() > 2 {
                    Some(arg[2..].to_owned())
                } else {
                    args.next()
                }
                    .ok_or_else(|| format!("missing argument to {}", arg))?
                    .parse()
                    .map_err(|e| format!("invalid argument to {}: {}", &arg[0..2], e))?;

                if !(2..=16).contains(&n) {
                    return Err(format!("argument to {} must be between 2 and 16 (inclusive)", &arg[0..2]));
                }

                if arg.starts_with("-i") {
                    opts.iradix = n;
                } else {
                    opts.oradix = n;
                }
            } else {
                return Err(format!("unrecognized argument {:?}", arg));
            }
        }
        Ok(opts)
    }
}

// Thin wrapper around DC4::action. We only expect DCResult::Continue, so turn any other result
// into an Err so we can use the question mark operator.
fn action(dc: &mut DC4, action: Action, w: &mut impl Write)
    -> Result<(), Result<dc4::DCResult, dc4::DCError>>
{
    match dc.action(action, w) {
        Ok(dc4::DCResult::Continue) => Ok(()),
        other => Err(other),
    }
}

fn run(r: impl Read, mut w: impl Write) -> Result<(), Result<dc4::DCResult, dc4::DCError>> {
    let mut dc = DC4::new("sum-numbers".to_owned());

    let opts = Options::parse(std::env::args())
        .unwrap_or_else(|e| {
            eprintln!("ERROR: {}", e);
            std::process::exit(2);
        });

    if opts.oradix != 10 {
        dc.push_number(&opts.oradix.to_string().into_bytes());
        action(&mut dc, Action::SetOutputRadix, &mut w)?;
    }
    if opts.iradix != 10 {
        dc.push_number(&opts.iradix.to_string().into_bytes());
        action(&mut dc, Action::SetInputRadix, &mut w)?;
    }

    // initial value
    dc.push_number("0");

    for result in Input::new(r) {
        match result {
            Ok(mut s) => {
                if s.starts_with(b"-") {
                    // dc uses '_' to designate negative numbers because '-' is used for
                    // subtraction, so replace it.
                    s[0] = b'_';
                }
                dc.push_number(&s);
                action(&mut dc, Action::Add, &mut w)?;
            }
            Err(e) => {
                eprintln!("ERROR: {}", e);
                std::process::exit(2);
            }
        }
    }

    action(&mut dc, Action::PrintStack, &mut w)?;
    Ok(())
}

fn main() {
    let w = io::stdout();
    let stdin = io::stdin();
    let stdin_lock = stdin.lock();

    if let Err(result) = run(stdin_lock, w) {
        panic!("unexpected result: {:?}", result);
    }
}
