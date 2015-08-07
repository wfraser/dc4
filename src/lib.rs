//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015 by William R. Fraser
//

use std::io::Read;
use std::io::Write;
use std::fmt;
use std::fmt::Arguments;

pub struct DC4 {
    program_name: String
}

pub enum DCResult {
    Terminate,
    QuitLevels(i32),
    Continue
}

fn loop_over_stream<S, F>(s: &mut S, mut f: F) -> DCResult
        where S: Read, F: FnMut(char) -> DCResult {
    // TODO: change this to s.chars() once Read::chars is stable
    for maybe_char in s.bytes() {
        match maybe_char {
            Ok(c)       => {
                match f(c as char) {
                    DCResult::Continue => (), // next loop iteration
                    other               => return other
                }
            },
            Err(err)    => {
                println!("Error reading from input: {}", err);
                return DCResult::Terminate;
            }
        }
    }
    DCResult::Continue
}

impl DC4 {
    pub fn new(program_name: String) -> DC4 {
        DC4 {
            program_name: program_name
        }
    }

    pub fn program<R, W>(&mut self, r: &mut R, w: &mut W) -> DCResult
            where R: Read,
            W: Write {
        loop_over_stream(r, |c| self.loop_iteration(c, w) )
    }

    fn loop_iteration<W>(&mut self, c: char, w: &mut W) -> DCResult
            where W: Write {
        //TODO
        match c {
            ' '|'\t'|'\r'|'\n' => (), // ignore whitespace

             // nonstandard extension: print the implementation name
            '@' => write!(w, "dc4\n").unwrap(),

            // catch-all for unhandled characters
            _ => self.error(w, format_args!("{:?} (0{:o}) unimplemented", c, c as u32))
        }
        DCResult::Continue
    }

    fn error<W>(&self, w: &mut W, args: Arguments) where W: Write {
        write!(w, "{}: {}\n", self.program_name, fmt::format(args)).unwrap();
    }
}
