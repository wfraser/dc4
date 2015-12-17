//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015 by William R. Fraser
//

extern crate num;

use std::io::Read;
use std::io::Write;
use std::fmt;
use std::fmt::Arguments;
use num::traits::FromPrimitive;
use num::{BigInt, Zero};

enum DCValue {
    Str(String),
    Num(BigInt)
}

pub struct DC4 {
    program_name: String,
    stack: Vec<DCValue>,
    scale: u32,
    iradix: u32,
    oradix: u32,
    input_num: Option<BigInt>,
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
                    other              => return other
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
    pub fn shaddap(self) {
        // Silence warnings about things that intentionally aren't used yet.
        (self.scale, DCValue::Str(String::new()));
    }

    pub fn new(program_name: String) -> DC4 {
        DC4 {
            program_name: program_name,
            stack: Vec::new(),
            scale: 0,
            iradix: 10,
            oradix: 10,
            input_num: Option::None,
        }
    }

    fn print_elem<W>(&self, elem: &DCValue, w: &mut W) where W: Write {
        match elem {
            &DCValue::Num(ref n) => write!(w, "{}\n", n.to_str_radix(self.oradix)).unwrap(),
            &DCValue::Str(ref s) => write!(w, "{}\n", s).unwrap(),
        }
    }

    fn print_stack<W>(&self, w: &mut W) where W: Write {
        for elem in self.stack.iter().rev() {
            self.print_elem(elem, w);
        }
    }

    pub fn program<R, W>(&mut self, r: &mut R, w: &mut W) -> DCResult
            where R: Read,
            W: Write {
        loop_over_stream(r, |c| self.loop_iteration(c, w) )
    }

    fn loop_iteration<W>(&mut self, c: char, w: &mut W) -> DCResult
            where W: Write {

        if (c >= '0' && c <= '9') || (c >= 'A' && c <= 'F') {
            if self.input_num.is_none() {
                self.input_num = Some(Zero::zero());
            }

            self.input_num = Some(
                self.input_num.as_ref().unwrap()
                * BigInt::from_u32(self.iradix).unwrap()
                + BigInt::from_u32(c.to_digit(16).unwrap()).unwrap()
                );

            //println!("digit: {:?}", self.input_num.as_ref().unwrap());

            return DCResult::Continue;
        }

        if !self.input_num.is_none() {
            //println!("pushing: {:?}", self.input_num.as_ref().unwrap());
            self.stack.push(DCValue::Num(self.input_num.take().unwrap()));
        }

        match c {
            ' '|'\t'|'\r'|'\n' => (), // ignore whitespace

             // nonstandard extension: print the implementation name
            '@' => write!(w, "dc4\n").unwrap(),

            'f' => self.print_stack(w),

            'p' => match self.stack.last() {
                Some(elem) => self.print_elem(elem, w),
                None => self.error(w, format_args!("stack empty")),
            },

            // catch-all for unhandled characters
            _ => self.error(w, format_args!("{:?} (0{:o}) unimplemented", c, c as u32))
        }
        DCResult::Continue
    }

    fn error<W>(&self, w: &mut W, args: Arguments) where W: Write {
        write!(w, "{}: {}\n", self.program_name, fmt::format(args)).unwrap();
    }
}
