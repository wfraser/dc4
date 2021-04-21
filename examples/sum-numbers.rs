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

use dc4::Dc4;
use dc4::parser::Action;
use std::io::{self, BufRead, Write};

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
fn action(dc: &mut Dc4, action: Action, w: &mut impl Write)
    -> Result<(), dc4::DCError>
{
    match dc.action(action, w) {
        Ok(dc4::DCResult::Continue) => Ok(()),
        Ok(other) => Err(format!("unexpected result: {:?}", other).into()),
        Err(other) => Err(other),
    }
}

fn run(r: impl BufRead, mut w: impl Write) -> Result<(), dc4::DCError> {
    let mut dc = Dc4::new("sum-numbers".to_owned());

    let opts = Options::parse(std::env::args())
        .unwrap_or_else(|e| {
            eprintln!("ERROR: {}", e);
            std::process::exit(2);
        });

    if opts.oradix != 10 {
        dc.push_number(&opts.oradix.to_string().into_bytes())?;
        action(&mut dc, Action::SetOutputRadix, &mut w)?;
    }
    if opts.iradix != 10 {
        dc.push_number(&opts.iradix.to_string().into_bytes())?;
        action(&mut dc, Action::SetInputRadix, &mut w)?;
    }

    // initial value
    dc.push_number("0").unwrap();

    //for result in Input::new(r) {
    for result in r.lines() {
        let s = result.map_err(|e| dc4::DCError::from(format!("I/O error: {}", e)))?;
        // dc uses '_' to designate negative numbers because '-' is used for subtraction, so
        // replace it.
        if let Err(e) = dc.push_number(s.replace('-', "_").trim()) {
            return Err(format!("invalid input {:?}: {}", s, e).into());
        }
        action(&mut dc, Action::Add, &mut w)?;
    }

    action(&mut dc, Action::PrintStack, &mut w)?;
    Ok(())
}

fn main() {
    let w = io::stdout();
    let stdin = io::stdin();
    let stdin_lock = stdin.lock();

    if let Err(result) = run(stdin_lock, w) {
        eprintln!("error: {}", result);
        std::process::exit(2);
    }
}
