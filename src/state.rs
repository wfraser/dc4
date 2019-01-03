//
// dc4 main program state
//
// Copyright (c) 2015-2019 by William R. Fraser
//

use std::io::{Read, Write};
use num::BigInt;
use num::traits::{ToPrimitive, Zero};
use big_real::BigReal;
use dcregisters::DCRegisters;
use parser::{Action, RegisterAction};
use crate::{DCValue, DCResult, DCError};

pub struct DC4 {
    program_name: String,
    stack: Vec<DCValue>,
    registers: DCRegisters,
    scale: u32,
    iradix: u32,
    oradix: u32,
}

impl DC4 {
    pub fn new(program_name: String) -> Self {
        Self {
            program_name,
            stack: vec![],
            registers: DCRegisters::new(),
            scale: 0,
            iradix: 10,
            oradix: 10,
        }
    }

    pub fn program(&mut self, input: &mut impl Read, w: &mut impl Write) -> DCResult {
        use parser::Parser;
        let mut input_decoder = utf8::BufReadDecoder::new(std::io::BufReader::new(input));
        let mut parser = Parser::new();
        loop {
            let buf: &str = match input_decoder.next_strict() {
                Some(Ok(s)) => s,
                None => return DCResult::Continue,
                Some(Err(utf8::BufReadDecoderError::Io(err))) => {
                    self.error(w, format_args!("error reading from input: {}", err));
                    return DCResult::Terminate(0);
                }
                Some(Err(utf8::BufReadDecoderError::InvalidByteSequence(bytes))) => {
                    self.error(w, format_args!("invalid UTF-8 in input: {:x?}", bytes));
                    "\u{FFFD}"
                }
            };

            let mut chars = buf.chars();
            let mut c = chars.next();
            loop {
                if let Some(action) = parser.step(&mut c) {
                    match self.action(action, w) {
                        Ok(DCResult::Continue) => (), // next loop iteration
                        Ok(DCResult::Recursion(_text)) => unimplemented!("recursion"),
                        Err(msg) => {
                            self.error(w, format_args!("{}", msg));
                        }
                        Ok(other) => {
                            return other;
                        }
                    }
                }
                if c.is_none() {
                    c = chars.next();
                    if c.is_none() {
                        break;
                    }
                }
            }
        }
    }

    pub fn action(&mut self, action: Action, w: &mut impl Write) -> Result<DCResult, DCError> {
        match action {
            Action::PushNumber(s) => {
                let mut n = BigInt::zero();
                let mut shift = None;
                let mut neg = false;
                for c in s.chars() {
                    match c {
                        '_' => { neg = true; }
                        '0' ... '9' | 'A' ... 'F' => {
                            n *= self.iradix;
                            n += c.to_digit(16).unwrap();
                            if let Some(shift) = shift.as_mut() {
                                *shift += 1;
                            }
                        }
                        '.' => { shift = Some(0); }
                        _ => unreachable!()
                    }
                }
                if neg {
                    n *= -1;
                }
                let mut real = BigReal::from(n);
                if let Some(shift) = shift {
                    if self.iradix == 10 {
                        // shortcut: shift is a number of decimal digits. The input was given in
                        // decimal, so just set the shift directly.
                        real.set_shift(shift);
                    } else {
                        // Otherwise, we have to repeatedly divide by iradix to get the right
                        // value. NOTE: the value 'shift' is the number of digits of input in
                        // whatever base iradix is. BigReal will interpret this as being decimal
                        // digits. THIS GOOFY NONSENSE IS WHAT dc ACTUALLY DOES. It can result in
                        // truncation of the input unless it had extra trailing zeroes on it. (try:
                        // "16i 1.F p" to see)
                        let divisor = BigReal::from(self.iradix);
                        for _ in 0 .. shift {
                            real = real.div(&divisor, shift);
                        }
                    }
                }
                self.stack.push(DCValue::Num(real));
            }
            Action::PushString(s) => {
                self.stack.push(DCValue::Str(s));
            }
            Action::Register(action, register) => {
                unimplemented!("action {:?} on register {:?}", action, register as char);
            }
            Action::Print => {
                match self.stack.last() {
                    Some(ref v) => self.print_elem(v, w),
                    None => return Err("stack empty".into())
                }
                writeln!(w).unwrap();
            }
            Action::PrintNoNewlinePop => {
                match self.stack.pop() {
                    Some(v) => self.print_elem(&v, w),
                    None => return Err("stack empty".into())
                }
            }

            _ => unimplemented!()
        }
        Ok(DCResult::Continue)
    }

    fn print_elem(&self, elem: &DCValue, w: &mut impl Write) {
        match *elem {
            DCValue::Num(ref n) => write!(w, "{}", n.to_str_radix(self.oradix).to_uppercase()),
            DCValue::Str(ref s) => write!(w, "{}", s),
        }.unwrap();
    }

    fn error(&self, w: &mut impl Write, args: std::fmt::Arguments) {
        writeln!(w, "{}: {}", self.program_name, std::fmt::format(args)).unwrap();
    }
}
