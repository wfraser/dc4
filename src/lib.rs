//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2017 by William R. Fraser
//

#![allow(unknown_lints, redundant_closure_call)]

use std::io::{self, Read, Write};
use std::fmt;
use std::mem;

extern crate num;
use num::traits::{ToPrimitive, Zero};
use num::BigInt;

mod big_real;
use big_real::BigReal;

mod dcregisters;
use dcregisters::DCRegisters;

pub struct DC4 {
    program_name: String,
    stack: Vec<DCValue>,
    registers: DCRegisters,
    scale: u32,
    iradix: u32,
    oradix: u32,
    input_num: Option<BigInt>,
    input_shift_digits: Option<u32>,
    input_str: String,
    bracket_level: u32,
    in_comment: bool,
    negative: bool,
    invert: bool,
    prev_char: char,
}

#[derive(Clone, Debug)]
pub enum DCValue {
    Str(String),
    Num(BigReal)
}

#[derive(Debug)]
pub enum DCResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Recursion(String),
}

pub enum DCError {
    Message(String),
    StaticMessage(&'static str),
}

impl std::fmt::Display for DCError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match *self {
            DCError::Message(ref msg) => msg,
            DCError::StaticMessage(msg) => msg,
        };
        f.write_str(msg)
    }
}

impl Into<DCError> for String {
    fn into(self) -> DCError {
        DCError::Message(self)
    }
}

impl Into<DCError> for &'static str {
    fn into(self) -> DCError {
        DCError::StaticMessage(self)
    }
}

fn read_byte<R: Read>(r: &mut R) -> Result<Option<u8>, std::io::Error> {
    let mut buf = [0u8; 1];
    let n = r.read(&mut buf)?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(buf[0]))
    }
}

fn read_char<R: Read>(r: &mut R) -> Result<Option<char>, String> {
    let first_byte: u8 = match read_byte(r).map_err(|e| format!("I/O error: {}", e))? {
        Some(byte) => byte,
        None => {
            return Ok(None);
        }
    };
    if first_byte < 0b1000_0000 {
        Ok(Some(first_byte as char))
    } else {
        let nbytes = if first_byte & 0b1110_0000 == 0b1100_0000 {
            2
        } else if first_byte & 0b1111_0000 == 0b1110_0000 {
            3
        } else if first_byte & 0b1111_1000 == 0b1111_0000 {
            4
        } else {
            // Illegal leading byte for UTF-8. Don't read any continuation bytes; just let
            // str::from_utf8 return an error.
            1
        };
        let mut bytes = Vec::with_capacity(nbytes);
        bytes.push(first_byte);
        for _ in 1..nbytes {
            match read_byte(r).map_err(|e| format!("I/O error: {}", e))? {
                Some(byte) => bytes.push(byte),
                None => break
            }
        }
        let s = std::str::from_utf8(&bytes)
                         .map_err(|e| format!("unable to parse {:?} as UTF-8: {}", bytes, e))?;
        Ok(Some(s.chars().next().unwrap()))
    }
}

fn loop_over_stream<R, F>(input: &mut R, mut f: F) -> Result<DCResult, DCError>
        where R: Read, F: FnMut(char) -> DCResult {
    // TODO: change this to use input.chars() once Read::chars is stable
    loop {
        match read_char(input) {
            Ok(Some(c)) => {
                match f(c as char) {
                    DCResult::Continue => (), // next loop iteration
                    other              => return Ok(other)
                }
            },
            Ok(None) => break,
            Err(err) => {
                return Err(format!("error reading from input: {}", err).into());
            }
        }
    }
    Ok(DCResult::Continue)
}

// Acts like a try block by running code in a closure.
macro_rules! capture_errors {
    ($block:block) => {
        (|| {
            Ok($block)
        })()
    }
}

impl DC4 {
    pub fn new(program_name: String) -> DC4 {
        DC4 {
            program_name: program_name,
            stack: Vec::new(),
            registers: DCRegisters::new(),
            scale: 0,
            iradix: 10,
            oradix: 10,
            input_num: None,
            input_shift_digits: None,
            input_str: String::new(),
            bracket_level: 0,
            in_comment: false,
            negative: false,    // for number entry
            invert: false,      // for conditional macro execution
            prev_char: '\0',
        }
    }

    fn print_elem<W>(&self, elem: &DCValue, w: &mut W) where W: Write {
        match *elem {
            DCValue::Num(ref n) => write!(w, "{}", n.to_str_radix(self.oradix).to_uppercase()).unwrap(),
            DCValue::Str(ref s) => write!(w, "{}", s).unwrap(),
        }
    }

    fn print_stack<W>(&self, w: &mut W) where W: Write {
        for elem in self.stack.iter().rev() {
            self.print_elem(elem, w);
            write!(w, "\n").unwrap();
        }
    }

    fn get_two_ints(&mut self) -> Result<(&BigReal, &BigReal), DCError> {
        let a: &BigReal;
        let b: &BigReal;

        let len = self.stack.len();
        if len < 2 {
            return Err("stack empty".into());
        }

        match self.stack[len - 2] {
            DCValue::Num(ref n) => { a = n; },
            _ => {
                return Err("non-numeric value".into());
            }
        }
        match self.stack[len - 1] {
            DCValue::Num(ref n) => { b = n; },
            _ => {
                return Err("non-numeric value".into());
            }
        }
        Ok((a, b))
    }

    fn binary_operator<F>(&mut self, mut f: F) -> Result<(), DCError>
            where F: FnMut(&BigReal, &BigReal) -> Result<Option<DCValue>, DCError> {

        let result: Result<Option<DCValue>, DCError> = {
            let (a, b) = self.get_two_ints()?;
            f(a, b)
        };

        match result {
            Ok(r) => {
                self.stack.pop();
                self.stack.pop();
                if let Some(value) = r {
                    self.stack.push(value);
                }
                Ok(())
            },
            Err(message) => {
                Err(message)
            }
        }
    }

    fn binary_operator2<F>(&mut self, mut f: F) -> Result<(), DCError>
            where F: FnMut(&BigReal, &BigReal) -> Result<Vec<DCValue>, DCError> {

        let maybe_results: Result<Vec<DCValue>, DCError> = {
            let (a, b) = self.get_two_ints()?;
            f(a, b)
        };

        match maybe_results {
            Ok(results) => {
                self.stack.pop();
                self.stack.pop();
                for result in results {
                    self.stack.push(result);
                }
                Ok(())
            },
            Err(msg) => {
                Err(msg)
            }
        }
    }

    fn binary_predicate<F>(&mut self, f: F) -> Result<bool, DCError>
            where F: Fn(&BigReal, &BigReal) -> Result<bool, DCError> {

        let mut result = false;
        self.binary_operator(|a, b| {
            f(a, b).map(|v| {
                result = v;
                None
            })
        })?;
        Ok(result)
    }

    fn pop_stack(&mut self) -> Result<DCValue, DCError> {
        self.stack.pop().ok_or_else(|| "stack empty".into())
    }

    fn pop_string(&mut self) -> Result<Option<String>, DCError> {
        let correct_type = match self.stack.last() {
            Some(&DCValue::Str(_)) => true,
            None => return Err("stack empty".into()),
            _ => false
        };

        if correct_type {
            match self.stack.pop() {
                Some(DCValue::Str(string)) => Ok(Some(string)),
                _ => unreachable!()
            }
        }
        else {
            Ok(None)
        }
    }

    pub fn run_macro_reg(&mut self, c: char) -> Result<DCResult, DCError> {
        let macro_string = match self.registers.get(c)?.value() {
            Some(&DCValue::Str(ref string)) => Some(string.clone()),
            None => return Err(format!("register '{}' (0{:o}) is empty", c, c as usize).into()),
            _ => None
        };
        Ok(match macro_string {
            Some(string) => DCResult::Recursion(string),
            _ => DCResult::Continue
        })
    }

    pub fn run_macro_str<W: Write>(&mut self, w: &mut W, mut macro_text: String) -> DCResult {
        self.prev_char = '\0';

        let mut pos = 0usize;
        let mut len = macro_text.len();
        let mut tail_recursion_levels = 0;
        while pos < len {
            // extract the char at pos
            let c = unsafe { macro_text.slice_unchecked(pos, len).chars().next().unwrap() };

            // Seek to the next char boundary.
            loop {
                pos += 1;
                if pos == len || macro_text.is_char_boundary(pos) {
                    break;
                }
            }

            let mut result = match self.loop_iteration(c, w) {
                Ok(result) => result,
                Err(msg) => {
                    self.error(w, format_args!("{}", msg));
                    DCResult::Continue
                }
            };

            while let DCResult::Recursion(sub_text) = result {
                // This loop iteration wants to call a macro.
                // The macro to run is returned, and we handle that here.

                // Tail recursion optimization:
                // if macro text is empty, replace it with sub_text and continue
                if pos == len {
                    macro_text = sub_text;
                    pos = 0;
                    len = macro_text.len();

                    tail_recursion_levels += 1;
                    result = DCResult::Continue;
                }
                else {
                    // otherwise we have to actually do recursion.
                    result = self.run_macro_str(w, sub_text);
                }
            }

            match result {
                DCResult::Recursion(_) => unreachable!(),
                DCResult::QuitLevels(n) => {
                    if n > tail_recursion_levels {
                        return DCResult::QuitLevels(n - tail_recursion_levels);
                    } else {
                        return DCResult::Continue;
                    }
                },
                DCResult::Terminate(n) => {
                    if n > tail_recursion_levels {
                        return DCResult::Terminate(n - tail_recursion_levels);
                    } else {
                        return DCResult::Continue;
                    }
                },
                DCResult::Continue => (),
            }
        }
        DCResult::Continue
    }

    pub fn program<R: Read, W: Write>(&mut self, r: &mut R, w: &mut W) -> DCResult {
        //let mut current_text = String::new();
        match loop_over_stream(r, |c| {
            //current_text.push(c);
            match self.loop_iteration(c, w) {
                Ok(DCResult::Recursion(text)) => self.run_macro_str(w, text),
                Err(msg) => {
                    //println!("at \"{}\"", current_text);
                    self.error(w, format_args!("{}", msg));
                    DCResult::Continue
                },
                Ok(other) => other,
            }
        }) {
            Ok(result) => result,
            Err(msg) => {
                self.error(w, format_args!("{}", msg));
                DCResult::Terminate(0)
            }
        }
    }

    fn loop_iteration<W: Write>(&mut self, c: char, w: &mut W) -> Result<DCResult, DCError> {
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
            return Ok(DCResult::Continue);
        }

        if self.in_comment {
            if c == '\n' {
                self.in_comment = false;
            }
            return Ok(DCResult::Continue);
        } else if c == '#' {
            self.in_comment = true;
            return Ok(DCResult::Continue);
        }

        // operations that need one more character to be read:
        let mut return_early: Option<DCResult> = Some(DCResult::Continue);
        let invert = self.invert;
        let ok = capture_errors!({ match self.prev_char {
            's' => {
                let value = self.pop_stack()?;
                self.registers.get_mut(c)?.set(value);
            },

            'l' => match self.registers.get(c)?.value() {
                Some(value) => self.stack.push(value.clone()),
                None => return Err(format!("register '{}' (0{:o}) is empty", c, c as usize).into()),
            },

            'S' => {
                let value = self.pop_stack()?;
                self.registers.get_mut(c)?.push(value);
            },

            'L' => match self.registers.get_mut(c)?.pop() {
                Some(value) => self.stack.push(value),
                None => return Err(format!("stack register '{}' (0{:o}) is empty", c, c as usize).into()),
            },

            '<' => if self.binary_predicate(move |a, b| Ok(invert != (b < a)))? {
                return_early = Some(self.run_macro_reg(c)?);
            },

            '>' => if self.binary_predicate(move |a, b| Ok(invert != (b > a)))? {
                return_early = Some(self.run_macro_reg(c)?);
            },

            '=' => if self.binary_predicate(move |a, b| Ok(invert != (b == a)))? {
                return_early = Some(self.run_macro_reg(c)?);
            },

            ':' => {
                if self.stack.len() < 2 {
                    return Err("stack empty".into());
                }
                else {
                    // this command pops the values regardless of whether the types are correct,
                    // unlike most other commands in dc.
                    let key_dcvalue = self.stack.pop().unwrap();
                    let value = self.stack.pop().unwrap();

                    let key: Option<BigReal> = match key_dcvalue {
                        DCValue::Num(key) => {
                            if key.is_negative() {
                                None
                            }
                            else {
                                Some(key)
                            }
                        },
                        _ => None
                    };
                    if key.is_none() {
                        return Err("array index must be a nonnegative integer".into())
                    }
                    else {
                        self.registers.get_mut(c)?.array_store(key.unwrap(), value);
                    }
                }
            },

            // this command also pops the value regardless of whether it's the correct type.
            ';' => match self.stack.pop() {
                Some(DCValue::Num(ref index)) if !index.is_negative() => {
                    let value = (*self.registers.get(c)?.array_load(index)).clone();
                    self.stack.push(value);
                },
                Some(_) => return Err("array index must be a nonnegative integer".into()),
                None => return Err("stack empty".into()),
            },

            _ => { return_early = None; }
        }});

        if self.prev_char != '!' {
            self.invert = false;
        } else if c != '>' && c != '<' && c != '=' {
            self.prev_char = '\0';
            self.invert = false;
            self.in_comment = true; // ignore the rest of the line
            return Err("running shell commands is not supported".into());
        }

        if let Some(other) = return_early {
            self.prev_char = '\0';
            ok?;
            return Ok(other);
        }
        ok?;

        if (c >= '0' && c <= '9') || (c >= 'A' && c <= 'F') {
            if self.input_num.is_none() {
                self.input_num = Some(BigInt::zero());
            }

            self.input_num = Some(
                self.input_num.as_ref().unwrap()
                * BigInt::from(self.iradix)
                + BigInt::from(c.to_digit(16).unwrap())
                );

            if let Some(mut shift) = self.input_shift_digits.as_mut() {
                *shift += 1;
            }

            //println!("digit: {:?}", self.input_num.as_ref().unwrap());

            return Ok(DCResult::Continue);
        }

        if c == '.' && self.input_shift_digits.is_none() {
            self.input_shift_digits = Some(0); // start shifting
            if self.input_num.is_none() {
                self.input_num = Some(BigInt::from(0));
            }
            return Ok(DCResult::Continue);
        }
        // if c is '.' and the shift has already been specified, then fall through to the block
        // below and push the current number; then set the shift again and keep reading the next
        // number.

        if let Some(mut n) = self.input_num.take() {
            //println!("pushing: {:?}", n);
            if self.negative {
                n = n * BigInt::from(-1);
            }
            let mut real = BigReal::from(n);
            if let Some(shift) = self.input_shift_digits {
                if self.iradix == 10 {
                    // shortcut: shift is a number of decimal digits. The input was given in
                    // decimal, so just set the shift directly.
                    real.set_shift(shift);
                } else {
                    // Otherwise, we have to repeatedly divide by iradix to get the right value.
                    // NOTE: the value 'shift' is the number of digits of input in whatever base
                    // iradix is. BigReal will interpret this as being decimal digits. THIS GOOFY
                    // NONSENSE IS WHAT dc ACTUALLY DOES. It can result in truncation of the input
                    // unless it had extra trailing zeroes on it. (try: "16i 1.F p" to see)
                    let divisor = BigReal::from(self.iradix);
                    for _ in 0..shift {
                        real = real.div(&divisor, shift);
                    }
                }
                self.input_shift_digits = None;
            }
            self.stack.push(DCValue::Num(real));
            self.negative = false;
        }
        else if self.negative {
            self.stack.push(DCValue::Num(BigReal::zero()));
            self.negative = false;
        }

        match c {
            ' '|'\t'|'\r'|'\n' => (), // ignore whitespace

            '!' => { self.invert = true; },

            '.' => {
                self.input_shift_digits = Some(0);
                self.input_num = Some(BigInt::from(0));
            },

            's'|'l'|'S'|'L'|'>'|'<'|'='|':'|';' => {}, // then handled above next time around.

            '_' => self.negative = true,

            '?' => {
                let mut line = String::new();
                if let Err(e) = io::stdin().read_line(&mut line) {
                    writeln!(w, "warning: error reading input: {}", e).unwrap();
                }

                let result = self.run_macro_str(w, line);
                match result {
                    DCResult::Recursion(_) => unreachable!(),
                    DCResult::Continue => (),
                    DCResult::QuitLevels(n) => {
                        if n > 1 {
                            return Ok(DCResult::QuitLevels(n-1));
                        }
                    },
                    DCResult::Terminate(n) => {
                        return Ok(DCResult::Terminate(n));
                    }
                }
            },

            // nonstandard extension: push the version formatted as 0xMMmmPPPP (M=major, m=minor,
            // p=patch), followed by the implementation name.
            '@' => {
                let ver = env!("CARGO_PKG_VERSION_MAJOR").parse::<u64>().unwrap() << 24
                        | env!("CARGO_PKG_VERSION_MINOR").parse::<u64>().unwrap() << 16
                        | env!("CARGO_PKG_VERSION_PATCH").parse::<u64>().unwrap();
                self.stack.push(DCValue::Num(BigReal::from(ver)));
                self.stack.push(DCValue::Str("dc4".to_owned()));
            },

            '[' => self.bracket_level += 1,

            'f' => self.print_stack(w),

            'p' => match self.stack.last() {
                Some(elem) => {
                    self.print_elem(elem, w);
                    write!(w, "\n").unwrap();
                },
                None => return Err("stack empty".into()),
            },

            'n' => match self.stack.pop() {
                Some(elem) => self.print_elem(&elem, w),
                None => return Err("stack empty".into()),
            },

            'P' => match self.stack.pop() {
                Some(DCValue::Str(s)) => { write!(w, "{}", s).unwrap(); },
                Some(DCValue::Num(n)) => {
                    let mut int = n.abs();
                    let divisor = BigReal::from(256);
                    while int.is_positive() {
                        let div_rem = int.div_rem(&divisor, self.scale);
                        let byte = div_rem.1.to_u8().unwrap();
                        write!(w, "{}", byte as char).unwrap();
                        int = div_rem.0;
                    }
                },
                None => return Err("stack empty".into()),
            },

            'c' => self.stack.clear(),
            'd' => if let Some(value) = self.stack.last().cloned() {
                self.stack.push(value);
            },
            'r' => if self.stack.len() >= 2 {
                let a = self.stack.pop().unwrap();
                let b = self.stack.pop().unwrap();
                self.stack.push(a);
                self.stack.push(b);
            }
            else {
                return Err("stack empty".into());
            },

            'i' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) =>
                    match n.to_u32() {
                        Some(radix) if radix >= 2 && radix <= 16 => {
                             self.iradix = radix;
                        },
                        _ => return Err("input base must be a number between 2 and 16 (inclusive)".into()),
                    },
                Some(DCValue::Str(_)) =>
                    return Err("input base must be a number between 2 and 16 (inclusive)".into()),
                None => return Err("stack empty".into()),
            },

            'o' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) =>
                    match n.to_u32() {
                        Some(radix) if radix >= 2 => {
                            self.oradix = radix;
                        },
                        Some(_) => return Err("output base must be a number greater than 1".into()),
                        _ => if n.to_i32().is_some() {
                                return Err("output base must be a number greater than 1".into());
                            } else {
                                return Err("error interpreting output base (overflow?)".into());
                            },
                    },
                Some(DCValue::Str(_)) =>
                    return Err("output base must be a number greater than 1".into()),
                None => return Err("stack empty".into()),
            },

            'k' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) =>
                    match n.to_u32() {
                        Some(scale) => {
                            self.scale = scale;
                        },
                        _ => if n.to_i32().is_some() {
                                return Err("scale must be a nonnegative number".into());
                            }
                            else {
                                return Err("error interpreting scale (overflow?)".into());
                            },
                    },
                Some(DCValue::Str(_)) => return Err("scale must be a nonnegative number".into()),
                None => return Err("stack empty".into()),
            },

            'I' => self.stack.push(DCValue::Num(BigReal::from(self.iradix))),
            'O' => self.stack.push(DCValue::Num(BigReal::from(self.oradix))),
            'K' => self.stack.push(DCValue::Num(BigReal::from(self.scale))),

            // pop top and execute as macro
            'x' => if let Some(string) = self.pop_string()? {
                return Ok(DCResult::Recursion(string));
            },

            '+' => self.binary_operator(|a, b| Ok(Some(DCValue::Num(a + b))))?,
            '-' => self.binary_operator(|a, b| Ok(Some(DCValue::Num(a - b))))?,
            '*' => self.binary_operator(|a, b| Ok(Some(DCValue::Num(a * b))))?,
            '/' => {
                let scale = self.scale;
                self.binary_operator(|a, b| {
                    if b.is_zero() {
                        Err("divide by zero".into())
                    }
                    else {
                        Ok(Some(DCValue::Num(a.div(b, scale))))
                    }
                })?
            },

            // remainder
            '%' => {
                let scale = self.scale;
                self.binary_operator(|a, b| {
                    if b.is_zero() {
                        Err("divide by zero".into())
                    }
                    else {
                        Ok(Some(DCValue::Num(a.rem(b, scale))))
                    }
                })?
            },

            // quotient and remainder
            '~' => {
                let scale = self.scale;
                self.binary_operator2(|a, b| {
                    if b.is_zero() {
                        Err("divide by zero".into())
                    }
                    else {
                        let div_rem = a.div_rem(b, scale);
                        Ok(vec![ DCValue::Num(div_rem.0), DCValue::Num(div_rem.1) ])
                    }
                })?
            },

            // exponentiate
            '^' => {
                let mut warn = false;
                let scale = self.scale;
                self.binary_operator(|base, exponent| {
                    if !exponent.is_integer() {
                        // have to print the warning outside the clousure
                        warn = true;
                    }

                    let result = base.pow(exponent, scale);
                    Ok(Some(DCValue::Num(result)))
                })?;
                if warn {
                    self.error(w, format_args!("warning: non-zero scale in exponent"));
                }
            },

            // modular exponentiation
            '|' => {
                if self.stack.len() >= 3 {
                    for (i, value) in self.stack[self.stack.len() - 3..].iter().enumerate() {
                        match *value {
                            DCValue::Num(ref n) => {
                                if i == 1 && n.is_negative() {
                                    return Err("negative exponent".into());
                                } else if i == 2 && n.is_zero() {
                                    return Err("remainder by zero".into());
                                }
                            },
                            _ => return Err("non-numeric value".into())
                        }
                    }
                } else {
                    return Err("stack empty".into());
                }

                let skip = self.stack.len() - 3;
                let mut values = self.stack.drain(skip..)
                    .map(|value| match value {
                        DCValue::Num(num) => num,
                        _ => unreachable!(),
                    })
                    .collect::<Vec<BigReal>>();

                let modulus = values.pop().unwrap();
                let exponent = values.pop().unwrap();
                let base = values.pop().unwrap();

                if !base.is_integer() {
                    self.error(w, format_args!("warning: non-zero scale in base"));
                }
                if !exponent.is_integer() {
                    self.error(w, format_args!("warning: non-zero scale in exponent"));
                }
                if !modulus.is_integer() {
                    self.error(w, format_args!("warning: non-zero scale in modulus"));
                }

                let result = BigReal::modexp(&base, &exponent, &modulus, self.scale).unwrap();
                self.stack.push(DCValue::Num(result));
            },

            // square root
            'v' => match self.stack.pop() {
                Some(DCValue::Num(n)) => {
                    if n.is_negative() {
                        return Err("square root of negative number".into());
                    } else if n.is_zero() {
                        self.stack.push(DCValue::Num(n));
                    } else {
                        let x = n.sqrt(self.scale).unwrap();
                        self.stack.push(DCValue::Num(x));
                    }
                },
                Some(_) => return Err("square root of nonnumeric attempted".into()),
                None => return Err("stack empty".into())
            },

            'z' => {
                let depth = self.stack.len();
                self.stack.push(DCValue::Num(BigReal::from(depth)));
            },

            'Q' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) if n.is_positive() => {
                    return Ok(DCResult::QuitLevels(n.to_u32().unwrap()));
                },
                Some(_) => return Err("Q command requires a number >= 1".into()),
                None => return Err("stack empty".into())
            },

            'q' => return Ok(DCResult::Terminate(2)),

            // catch-all for unhandled characters
            _ => return Err(format!("{:?} (0{:o}) unimplemented", c, c as u32).into())
        }
        self.prev_char = c;

        Ok(DCResult::Continue)
    }

    fn error<W>(&self, w: &mut W, args: fmt::Arguments) where W: Write {
        write!(w, "{}: {}\n", self.program_name, fmt::format(args)).unwrap();
    }
}
