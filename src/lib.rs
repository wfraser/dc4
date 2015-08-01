//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015 by William R. Fraser
//

use std::io::Read;
use std::io::Write;

pub struct DC4;

pub enum DC4Result {
    Terminate,
    QuitLevels(i32),
    Continue
}

fn loop_over_stream<S, F>(s: &mut S, mut f: F) -> DC4Result
        where S: Read, F: FnMut(char) -> DC4Result {
    // TODO: change this to s.chars() once Read::chars is stable
    for maybe_char in s.bytes() {
        match maybe_char {
            Ok(c)       => {
                match f(c as char) {
                    DC4Result::Continue => (), // next loop iteration
                    other               => return other
                }
            },
            Err(err)    => {
                println!("Error reading from input: {}", err);
                return DC4Result::Terminate;
            }
        }
    }
    DC4Result::Continue
}

impl DC4 {
    pub fn new() -> DC4 {
        DC4
    }

    pub fn program<R, W>(&mut self, r: &mut R, w: &mut W) -> DC4Result
            where R: Read,
            W: Write {
        loop_over_stream(r, |c| self.loop_iteration(c, w) )
    }

    fn loop_iteration<W>(&mut self, c: char, w: &mut W) -> DC4Result
            where W: Write {
        //TODO
        w.write("TODO\n".as_bytes()).unwrap();
        DC4Result::Continue
    }
}
