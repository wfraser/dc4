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
use std::mem;
use num::traits::{FromPrimitive, ToPrimitive};
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
    input_str: String,
    bracket_level: u32,
    negative: bool,
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
        (self.scale);
    }

    pub fn new(program_name: String) -> DC4 {
        DC4 {
            program_name: program_name,
            stack: Vec::new(),
            scale: 0,
            iradix: 10,
            oradix: 10,
            input_num: Option::None,
            input_str: String::new(),
            bracket_level: 0,
            negative: false,
        }
    }

    fn print_elem<W>(&self, elem: &DCValue, w: &mut W) where W: Write {
        match elem {
            &DCValue::Num(ref n) => write!(w, "{}", n.to_str_radix(self.oradix).to_uppercase()).unwrap(),
            &DCValue::Str(ref s) => write!(w, "{}", s).unwrap(),
        }
    }

    fn print_stack<W>(&self, w: &mut W) where W: Write {
        for elem in self.stack.iter().rev() {
            self.print_elem(elem, w);
            write!(w, "\n").unwrap();
        }
    }

    pub fn program<R, W>(&mut self, r: &mut R, w: &mut W) -> DCResult
            where R: Read,
            W: Write {
        loop_over_stream(r, |c| self.loop_iteration(c, w) )
    }

    fn loop_iteration<W>(&mut self, c: char, w: &mut W) -> DCResult
            where W: Write {

        if self.bracket_level > 0 {
            if c == '[' {
                self.bracket_level += 1;
            }
            else if c == ']' {
                self.bracket_level -= 1;
                if self.bracket_level == 0 {
                    let mut value = String::new();
                    mem::swap(&mut value, &mut self.input_str);
                    self.stack.push(DCValue::Str(value));
                }
            }

            if self.bracket_level > 0 {
                self.input_str.push(c);
            }
            return DCResult::Continue;
        }

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
            let mut n = self.input_num.take().unwrap();
            if self.negative {
                n = n * BigInt::from_i32(-1).unwrap();
            }
            self.stack.push(DCValue::Num(n));
            self.negative = false;
        }
        else if self.negative {
            self.stack.push(DCValue::Num(Zero::zero()));
            self.negative = false;
        }

        match c {
            ' '|'\t'|'\r'|'\n' => (), // ignore whitespace

            '_' => self.negative = true,

             // nonstandard extension: print the implementation name
            '@' => write!(w, "dc4\n").unwrap(),

            '[' => self.bracket_level += 1,

            'f' => self.print_stack(w),

            'p' => match self.stack.last() {
                Some(elem) => {
                    self.print_elem(elem, w);
                    write!(w, "\n").unwrap();
                },
                None => self.error(w, format_args!("stack empty")),
            },

            'n' => match self.stack.pop() {
                Some(elem) => self.print_elem(&elem, w),
                None => self.error(w, format_args!("stack empty")),
            },

            'i' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) =>
                    match n.to_u32() {
                        Some(radix) if radix >= 2 && radix <= 16 => {
                             self.iradix = radix;
                        },
                        _ => self.error(w, format_args!("input base must be a number between 2 and 16 (inclusive)")),
                    },
                Some(DCValue::Str(_)) =>
                    self.error(w, format_args!("input base must be a number between 2 and 16 (inclusive)")),
                None => self.error(w, format_args!("stack empty")),
            },

            'o' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) =>
                    match n.to_u32() {
                        Some(radix) if radix >= 2 => {
                            self.oradix = radix;
                        },
                        Some(_) => self.error(w, format_args!("output base must be a number greater than 1")),
                        _ => if let Some(_) = n.to_i32() {
                                self.error(w, format_args!("output base must be a number greater than 1"));
                            } else {
                                self.error(w, format_args!("error interpreting output base (overflow?)"));
                            },
                    },
                Some(DCValue::Str(_)) =>
                    self.error(w, format_args!("output base must be a number greater than 1")),
                None => self.error(w, format_args!("stack empty")),
            },

            'k' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) =>
                    match n.to_u32() {
                        Some(scale) => {
                            self.scale = scale;
                        },
                        _ => if let Some(_) = n.to_i32() {
                                self.error(w, format_args!("scale must be a nonnegative number"));
                            }
                            else {
                                self.error(w, format_args!("error interpreting scale (overflow?)"));
                            },
                    },
                Some(DCValue::Str(_)) => self.error(w, format_args!("scale must be a nonnegative number")),
                None => self.error(w, format_args!("stack empty")),
            },

            'I' => self.stack.push(DCValue::Num(BigInt::from_u32(self.iradix).unwrap())),
            'O' => self.stack.push(DCValue::Num(BigInt::from_u32(self.oradix).unwrap())),
            'K' => self.stack.push(DCValue::Num(BigInt::from_u32(self.scale).unwrap())),

            // catch-all for unhandled characters
            _ => self.error(w, format_args!("{:?} (0{:o}) unimplemented", c, c as u32))
        }
        DCResult::Continue
    }

    fn error<W>(&self, w: &mut W, args: Arguments) where W: Write {
        write!(w, "{}: {}\n", self.program_name, fmt::format(args)).unwrap();
    }
}
