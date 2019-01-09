//
// sum-numbers :: Accumulate and sum whitespace-delimited numbers from input.
//
// Copyright (c) 2019 by William R. Fraser
//

/// This is an example of how DC4 can be used as a library for doing useful numeric operations.
/// The program reads numbers from input, delimited by whitespace, and uses DC4 to add them up as
/// it reads them. When it reaches EOF, it prints the resulting sum. Because it uses DC4, it
/// supports arbitrary precision.

extern crate dc4;
use dc4::DC4;
use dc4::parser::Action;
use std::io::{self, BufRead, Write};

struct Input<R: BufRead> {
    inner: R,
    buf: String,
    decimal: bool,
}

impl<R: BufRead> Input<R> {
    pub fn new(byte_reader: R) -> Self {
        Self {
            inner: byte_reader,
            buf: String::new(),
            decimal: false,
        }
    }
}

impl<R: BufRead> Iterator for Input<R> {
    type Item = io::Result<String>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // The only valid input to this program is in [0-9.-] which are all ASCII, so this
            // simply reads input by bytes. It'd be easy enough to read UTF-8 instead, though, and
            // that's what dc's parser does.
            let mut buf = [0u8];
            let c = match self.inner.read(&mut buf) {
                Ok(0) => None,
                Ok(1) => Some(Ok(buf[0] as char)),
                Ok(_) => unreachable!(),
                Err(e) => Some(Err(e)),
            };
            match c {
                None => {
                    if self.buf.is_empty() {
                        return None;
                    } else {
                        self.decimal = false;
                        return Some(Ok(std::mem::replace(&mut self.buf, String::new())));
                    }
                }
                Some(Err(e)) => {
                    return Some(Err(e));
                }
                Some(Ok(c)) => {
                    if c.is_whitespace() {
                        if !self.buf.is_empty() {
                            self.decimal = false;
                            return Some(Ok(std::mem::replace(&mut self.buf, String::new())));
                        }
                    } else if c == '.' {
                        if self.decimal {
                            return Some(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("invalid number in input: \"{}{}\"", self.buf, c))));
                        } else {
                            self.decimal = true;
                            self.buf.push(c);
                        }
                    } else if c == '-' {
                        if self.buf.is_empty() {
                            self.buf.push(c);
                        } else {
                            return Some(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("invalid number in input: \"{}{}\"", self.buf, c))));
                        }
                    } else if c >= '0' && c <= '9' {
                        self.buf.push(c);
                    } else {
                        return Some(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("invalid character in input: {:?}", c))));
                    }
                }
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

                if n < 2 || n > 16 {
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

fn run(r: impl BufRead, mut w: impl Write) -> Result<(), Result<dc4::DCResult, dc4::DCError>> {
    let mut dc = DC4::new("sum-numbers".to_owned());

    let opts = Options::parse(std::env::args())
        .unwrap_or_else(|e| {
            eprintln!("ERROR: {}", e);
            std::process::exit(2);
        });

    if opts.oradix != 10 {
        action(&mut dc, Action::PushNumber(opts.oradix.to_string()), &mut w)?;
        action(&mut dc, Action::SetOutputRadix, &mut w)?;
    }
    if opts.iradix != 10 {
        action(&mut dc, Action::PushNumber(opts.iradix.to_string()), &mut w)?;
        action(&mut dc, Action::SetInputRadix, &mut w)?;
    }

    // initial value
    action(&mut dc, Action::PushNumber("0".to_owned()), &mut w)?;

    for result in Input::new(r) {
        match result {
            Ok(mut s) => {
                if s.starts_with('-') {
                    // dc uses '_' to designate negative numbers because '-' is used for
                    // subtraction, so replace it.
                    unsafe {
                        // safe because we're replacing a 1-byte character with another.
                        s.as_bytes_mut()[0] = b'_';
                    }
                }
                action(&mut dc, Action::PushNumber(s), &mut w)?;
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
