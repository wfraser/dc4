//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015-2016 by William R. Fraser
//

use std::io::{Read, Write};
use std::collections::HashMap;
use std::fmt;
use std::mem;
use std::rc::Rc;

extern crate num;
use num::traits::{ToPrimitive, Zero, One};
use num::BigInt;

mod big_real;
use big_real::BigReal;

const MAX_REGISTER: usize = 255;

#[derive(Clone, Debug)]
enum DCValue {
    Str(String),
    Num(BigReal)
}

struct DCRegister {
    main_value: Option<DCValue>,
    map: HashMap<BigReal, Rc<DCValue>>,
}

impl DCRegister {
    pub fn new(value: Option<DCValue>) -> DCRegister {
        DCRegister {
            main_value: value,
            map: HashMap::new(),
        }
    }

    pub fn map_lookup(&self, key: &BigReal) -> Option<&Rc<DCValue>> {
        self.map.get(key)
    }

    pub fn map_insert(&mut self, key: BigReal, value: DCValue) {
        self.map.insert(key, Rc::new(value));
    }
}

struct DCRegisterStack {
    stack: Vec<DCRegister>,
}

impl DCRegisterStack {
    pub fn new() -> DCRegisterStack {
        DCRegisterStack {
            stack: Vec::new()
        }
    }

    pub fn value(&self) -> Option<&DCValue> {
        match self.stack.last() {
            Some(reg) => match reg.main_value {
                Some(ref value) => Some(value),
                None        => None,
            },
            None      => None,
        }
    }

    pub fn array_store(&mut self, key: BigReal, value: DCValue) {
        if self.stack.is_empty() {
            self.stack.push(DCRegister::new(None));
        }
        self.stack.last_mut().unwrap().map_insert(key, value);
    }

    pub fn array_load(&self, key: &BigReal) -> Rc<DCValue> {
        match self.stack.last() {
            Some(reg) => match reg.map_lookup(key) {
                Some(value) => value.clone(),
                None        => Rc::new(DCValue::Num(BigReal::zero()))
            },
            None      => Rc::new(DCValue::Num(BigReal::zero()))
        }
    }

    pub fn set(&mut self, value: DCValue) {
        if !self.stack.is_empty() {
            self.stack.pop();
        }
        self.stack.push(DCRegister::new(Some(value)));
    }

    pub fn pop(&mut self) -> Option<DCValue> {
        self.stack.pop().and_then(|v| v.main_value)
    }

    pub fn push(&mut self, value: DCValue) {
        self.stack.push(DCRegister::new(Some(value)))
    }
}

pub struct DC4 {
    program_name: String,
    stack: Vec<DCValue>,
    registers: Vec<DCRegisterStack>,
    scale: u32,
    iradix: u32,
    oradix: u32,
    input_num: Option<BigInt>,
    input_shift_digits: Option<u32>,
    input_str: String,
    bracket_level: u32,
    negative: bool,
    invert: bool,
    prev_char: char,
}

#[derive(Debug)]
pub enum DCResult {
    Terminate(u32),
    QuitLevels(u32),
    Continue,
    Recursion(String),
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

fn read_char<R: Read>(r: &mut R) -> Result<Option<char>, Box<std::error::Error>> {
    let first_byte: u8 = match read_byte(r)? {
        Some(byte) => byte,
        None => {
            return Ok(None);
        }
    };
    if first_byte < 0b10000000 {
        Ok(Some(first_byte as char))
    } else {
        let nbytes = if first_byte & 0b1110_0000 == 0b1100_0000 {
            1
        } else if first_byte & 0b1111_0000 == 0b1110_0000 {
            2
        } else if first_byte & 0b1111_1000 == 0b1111_0000 {
            3
        } else {
            // Illegal leading byte for UTF-8. Don't read any continuation bytes; just let
            // str::from_utf8 return an error.
            0
        };
        let mut bytes = Vec::with_capacity(nbytes + 1);
        bytes.push(first_byte);
        for _ in 0..nbytes {
            match read_byte(r)? {
                Some(byte) => bytes.push(byte),
                None => break
            }
        }
        let s = std::str::from_utf8(&bytes)?;
        Ok(Some(s.chars().next().unwrap()))
    }
}

fn loop_over_stream<R, F>(input: &mut R, mut f: F) -> DCResult
        where R: Read, F: FnMut(char) -> DCResult {
    // TODO: change this to use input.chars() once Read::chars is stable
    loop {
        match read_char(input) {
            Ok(Some(c)) => {
                match f(c as char) {
                    DCResult::Continue => (), // next loop iteration
                    other              => return other
                }
            },
            Ok(None) => break,
            Err(err) => {
                panic!("Error reading from input: {}", err);
            }
        }
    }
    DCResult::Continue
}

impl DC4 {
    pub fn new(program_name: String) -> DC4 {
        let mut value = DC4 {
            program_name: program_name,
            stack: Vec::new(),
            registers: Vec::with_capacity(MAX_REGISTER + 1),
            scale: 0,
            iradix: 10,
            oradix: 10,
            input_num: None,
            input_shift_digits: None,
            input_str: String::new(),
            bracket_level: 0,
            negative: false,    // for number entry
            invert: false,      // for conditional macro execution
            prev_char: '\0',
        };
        for _ in 0 .. MAX_REGISTER + 1 {
            value.registers.push(DCRegisterStack::new());
        }
        value
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

    fn get_two_ints<W>(&mut self, w: &mut W) -> Option<(&BigReal, &BigReal)> where W: Write {
        let a: &BigReal;
        let b: &BigReal;

        let len = self.stack.len();
        if len < 2 {
            self.error(w, format_args!("stack empty"));
            return None;
        }

        match self.stack[len - 2] {
            DCValue::Num(ref n) => { a = &n; },
            _ => {
                self.error(w, format_args!("non-numeric value"));
                return None;
            }
        }
        match self.stack[len - 1] {
            DCValue::Num(ref n) => { b = &n; },
            _ => {
                self.error(w, format_args!("non-numeric value"));
                return None;
            }
        }
        Some((a, b))
    }

    fn binary_operator<W, F>(&mut self, w: &mut W, mut f: F)
            where W: Write,
            F: FnMut(&BigReal, &BigReal) -> Result<Option<DCValue>, String> {

        let result: Result<Option<DCValue>, String>;

        match self.get_two_ints(w) {
            Some((a, b)) => { result = f(a, b); },
            None => return,
        }

        match result {
            Ok(r) => {
                self.stack.pop();
                self.stack.pop();
                match r {
                    Some(value) => { self.stack.push(value); },
                    _ => {},
                }
            },
            Err(message) => {
                self.error(w, format_args!("{}", message));
            }
        }
    }

    fn binary_operator2<W, F>(&mut self, w: &mut W, mut f: F)
            where W: Write,
            F: FnMut(&BigReal, &BigReal) -> Result<Vec<DCValue>, String> {

        let maybe_results: Result<Vec<DCValue>, String>;

        match self.get_two_ints(w) {
            Some((a, b)) => { maybe_results = f(a, b); },
            None => return,
        }

        match maybe_results {
            Ok(results) => {
                self.stack.pop();
                self.stack.pop();
                for result in results {
                    self.stack.push(result);
                }
            },
            Err(msg) => {
                self.error(w, format_args!("{}", msg));
            }
        }
    }

    fn binary_predicate<W, F>(&mut self, w: &mut W, f: F) -> bool
            where W: Write,
            F: Fn(&BigReal, &BigReal) -> Result<bool, String> {

        let mut result = false;
        self.binary_operator(w, |a, b| {
            f(a, b).map(|v| {
                result = v;
                None
            })
        });
        result
    }

    fn pop_stack<W>(&mut self, w: &mut W) -> Option<DCValue> where W: Write {
        self.stack.pop().or_else(|| {
            self.error(w, format_args!("stack empty"));
            None
        })
    }

    fn pop_string<W>(&mut self, w: &mut W) -> Option<String> where W: Write {
        let correct_type = match self.stack.last() {
            Some(&DCValue::Str(_)) => true,
            None => {
                self.error(w, format_args!("stack empty"));
                false
            }
            _ => false
        };

        if correct_type {
            match self.stack.pop() {
                Some(DCValue::Str(string)) => Some(string),
                _ => unreachable!()
            }
        }
        else {
            None
        }
    }

    pub fn run_macro_reg<W>(&mut self, w: &mut W, c: char) -> DCResult where W: Write {
        let macro_string = match self.registers[c as usize].value() {
            Some(&DCValue::Str(ref string)) => Some(string.clone()),
            None => {
                self.error(w, format_args!("register '{}' (0{:o}) is empty", c, c as usize));
                None
            }
            _ => None
        };
        match macro_string {
            Some(string) => DCResult::Recursion(string),
            _ => DCResult::Continue
        }
    }

    pub fn run_macro_str<W>(&mut self, w: &mut W, macro_text: String) -> DCResult where W: Write {
        self.prev_char = '\0';

        let mut current_text = macro_text.into_bytes();
        let mut pos = 0;
        let mut len = current_text.len();
        let mut tail_recursion_levels = 0;
        while pos < len {
            let c = current_text[pos] as char;
            pos += 1;

            let mut result = self.loop_iteration(c, w);

            while let DCResult::Recursion(sub_text) = result {
                // This loop iteration wants to call a macro.
                // The macro to run is returned, and we handle that here.

                // Tail recursion optimization:
                // if macro text is empty, replace it with sub_text and continue
                if pos == len {
                    current_text = sub_text.into_bytes();
                    pos = 0;
                    len = current_text.len();
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

    pub fn program<R: Read, W: Write>(&mut self, r: &mut R, w: &mut W)
            -> DCResult {
        loop_over_stream(r, |c| match self.loop_iteration(c, w) {
            DCResult::Recursion(text) => self.run_macro_str(w, text),
            other                     => other
        })
    }

    fn loop_iteration<W: Write>(&mut self, c: char, w: &mut W) -> DCResult {
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

        // operations that need one more character to be read:
        let mut return_early: Option<DCResult> = Some(DCResult::Continue);
        let invert = self.invert;
        match self.prev_char {
            's' => if let Some(value) = self.pop_stack(w) {
                self.registers[c as usize].set(value);
            },

            'l' => match self.registers[c as usize].value() {
                Some(value) => self.stack.push(value.clone()),
                None => self.error(w, format_args!("register '{}' (0{:o}) is empty", c, c as usize)),
            },

            'S' => if let Some(value) = self.pop_stack(w) {
                self.registers[c as usize].push(value);
            },

            'L' => match self.registers[c as usize].pop() {
                Some(value) => self.stack.push(value),
                None => self.error(w, format_args!("stack register '{}' (0{:o}) is empty", c, c as usize)),
            },

            '<' => if self.binary_predicate(w, move |a, b| Ok(invert != (b < a))) {
                return_early = Some(self.run_macro_reg(w, c));
            },

            '>' => if self.binary_predicate(w, move |a, b| Ok(invert != (b > a))) {
                return_early = Some(self.run_macro_reg(w, c));
            },

            '=' => if self.binary_predicate(w, move |a, b| Ok(invert != (b == a))) {
                return_early = Some(self.run_macro_reg(w, c));
            },

            ':' => {
                if self.stack.len() < 2 {
                    self.error(w, format_args!("stack empty"));
                }
                else {
                    // this command pops the values regardless of whether the types are correct,
                    // unlike most other commands in dc.
                    /*
                    let type_match = match self.stack.last().unwrap() {
                        &DCValue::Num(ref n) => !n.is_negative(),
                        _                    => false
                    };
                    if type_match {
                        match self.stack.pop().unwrap() {
                            DCValue::Num(key) => {
                                let value = self.stack.pop().unwrap();
                                self.registers[c as usize].array_store(key, value);
                            },
                            _ => unreachable!()
                        }
                    }
                    else {
                        self.error(w, format_args!("array index must be a nonnegative integer"));
                    }
                    */
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
                        self.error(w, format_args!("array index must be a nonnegative integer"))
                    }
                    else {
                        self.registers[c as usize].array_store(key.unwrap(), value);
                    }
                }
            },

            // this command also pops the value regardless of whether it's the correc type.
            ';' => match self.stack.pop() {
                Some(DCValue::Num(ref index)) if !index.is_negative() => {
                    let ref value = *self.registers[c as usize].array_load(&index);
                    self.stack.push(value.clone());
                },
                Some(_) => self.error(w, format_args!("array index must be a nonnegative integer")),
                None => self.error(w, format_args!("stack empty")),
            },

            _ => { return_early = None; }
        };

        if self.prev_char != '!' {
            self.invert = false;
        }

        if let Some(other) = return_early {
            self.prev_char = '\0';
            return other;
        }

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

            return DCResult::Continue;
        }

        if c == '.' && self.input_shift_digits.is_none() {
            self.input_shift_digits = Some(0); // start shifting
            return DCResult::Continue;
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

            '.' => { self.input_shift_digits = Some(0); },

            's'|'l'|'S'|'L'|'>'|'<'|'='|':'|';' => {}, // then handled above next time around.

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

            'P' => match self.stack.pop() {
                Some(DCValue::Str(s)) => { write!(w, "{}", s).unwrap(); },
                Some(DCValue::Num(n)) => {
                    let mut int = n.abs();
                    while int.is_positive() {
                        let div_rem = int.div_rem(&BigReal::from(256), self.scale);
                        let byte = div_rem.1.to_u8().unwrap();
                        write!(w, "{}", byte as char).unwrap();
                        int = div_rem.0;
                    }
                },
                None => { self.error(w, format_args!("stack empty")); },
            },

            'c' => self.stack.clear(),
            'd' => if let Some(value) = self.stack.last().map(|v| v.clone()) {
                self.stack.push(value);
            },
            'r' => if self.stack.len() >= 2 {
                let a = self.stack.pop().unwrap();
                let b = self.stack.pop().unwrap();
                self.stack.push(a);
                self.stack.push(b);
            }
            else {
                self.error(w, format_args!("stack empty"));
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

            'I' => self.stack.push(DCValue::Num(BigReal::from(self.iradix))),
            'O' => self.stack.push(DCValue::Num(BigReal::from(self.oradix))),
            'K' => self.stack.push(DCValue::Num(BigReal::from(self.scale))),

            // pop top and execute as macro
            'x' => match self.pop_string(w).and_then(|string| Some(DCResult::Recursion(string))) {
                Some(result) => return result,
                None         => ()
            },

            '+' => self.binary_operator(w, |a, b| Ok(Some(DCValue::Num(a + b)))),
            '-' => self.binary_operator(w, |a, b| Ok(Some(DCValue::Num(a - b)))),
            '*' => self.binary_operator(w, |a, b| Ok(Some(DCValue::Num(a * b)))),
            '/' => {
                let scale = self.scale;
                self.binary_operator(w, |a, b| {
                    if b.is_zero() {
                        Err(format!("divide by zero"))
                    }
                    else {
                        Ok(Some(DCValue::Num(a.div(b, scale))))
                    }
                });
            },

            // remainder
            '%' => {
                let scale = self.scale;
                self.binary_operator(w, |a, b| {
                    if b.is_zero() {
                        Err(format!("divide by zero"))
                    }
                    else {
                        Ok(Some(DCValue::Num(a.rem(b, scale))))
                    }
                })
            },

            // quotient and remainder
            '~' => {
                let scale = self.scale;
                self.binary_operator2(w, |a, b| {
                    if b.is_zero() {
                        Err(format!("divide by zero"))
                    }
                    else {
                        let div_rem = a.div_rem(b, scale);
                        Ok(vec![ DCValue::Num(div_rem.0), DCValue::Num(div_rem.1) ])
                    }
                })
            },

            // exponentiate
            '^' => self.binary_operator(w, |base, exponent| {
                let mut result: BigReal;
                if exponent.is_zero() {
                    result = BigReal::one();
                }
                else if exponent.is_negative() {
                    result = BigReal::zero();
                }
                else {
                    result = base.clone();
                    for _ in num::iter::range(BigReal::zero(), exponent - BigReal::one()) {
                        result = result * base;
                    }
                }
                Ok(Some(DCValue::Num(result)))
            }),

            //TODO:
            // '|': modular exponentiation
            // 'v': square root

            'z' => {
                let depth = self.stack.len();
                self.stack.push(DCValue::Num(BigReal::from(depth)));
            },

            'Q' => match self.stack.pop() {
                Some(DCValue::Num(ref n)) if n.is_positive() => {
                    return DCResult::QuitLevels(n.to_u32().unwrap());
                },
                Some(_) => self.error(w, format_args!("Q command requires a number >= 1")),
                None => self.error(w, format_args!("stack empty"))
            },

            'q' => return DCResult::Terminate(2),

            // catch-all for unhandled characters
            _ => self.error(w, format_args!("{:?} (0{:o}) unimplemented", c, c as u32))
        }
        self.prev_char = c;

        DCResult::Continue
    }

    fn error<W>(&self, w: &mut W, args: fmt::Arguments) where W: Write {
        write!(w, "{}: {}\n", self.program_name, fmt::format(args)).unwrap();
    }
}
