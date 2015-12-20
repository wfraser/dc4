//
// dc4 :: A Unix dc(1) implementation in Rust
//
// Copyright (c) 2015 by William R. Fraser
//

extern crate num;

use std::io::Read;
use std::io::Write;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Arguments;
use std::mem;
use std::rc::Rc;
use num::traits::{ToPrimitive, Zero, One, Signed};
use num::{BigInt, Integer};
use num::iter::range;

mod option_then;
use option_then::OptionThen;

#[derive(Clone)]
enum DCValue {
    Str(String),
    Num(BigInt)
}

struct DCRegister {
    main_value: DCValue,
    map: HashMap<BigInt, Rc<DCValue>>,
}

impl DCRegister {
    pub fn new(value: DCValue) -> DCRegister{
        DCRegister {
            main_value: value,
            map: HashMap::new(),
        }
    }

    pub fn map_lookup(&self, key: &BigInt) -> Option<&Rc<DCValue>> {
        self.map.get(key)
    }

    pub fn map_insert(&mut self, key: BigInt, value: DCValue) {
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
            Some(reg) => Some(&reg.main_value),
            None      => None,
        }
    }

    pub fn set(&mut self, value: DCValue) {
        if !self.stack.is_empty() {
            self.stack.pop();
        }
        self.stack.push(DCRegister::new(value));
    }

    pub fn pop(&mut self) -> Option<DCValue> {
        if self.stack.is_empty() {
            None
        }
        else {
            Some(self.stack.pop().unwrap().main_value)
        }
    }

    pub fn push(&mut self, value: DCValue) {
        self.stack.push(DCRegister::new(value))
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
        where S: Read, F: FnMut(char, char) -> DCResult {
    // TODO: change this to s.chars() once Read::chars is stable
    let mut prev = '\0';
    for maybe_char in s.bytes() {
        match maybe_char {
            Ok(c)       => {
                match f(c as char, prev) {
                    DCResult::Continue => (), // next loop iteration
                    other              => return other
                }
                prev = c as char;
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
        let mut value = DC4 {
            program_name: program_name,
            stack: Vec::new(),
            registers: Vec::with_capacity(256),
            scale: 0,
            iradix: 10,
            oradix: 10,
            input_num: Option::None,
            input_str: String::new(),
            bracket_level: 0,
            negative: false,
        };
        for _ in range(0, 256) {
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

    /*
    fn pop_num<W>(&mut self, w: &mut W) -> Option<BigInt>
            where W: Write {
        match self.stack.last() {
            Some(&DCValue::Num(_)) => {
                // type match, pop and return the moved value
                match self.stack.pop() {
                    Some(DCValue::Num(n)) => Some(n),
                    _ => panic!("impossible!"),
                }
            }
            None => {
                self.error(w, format_args!("stack empty"));
                None
            },
            _ => {
                self.error(w, format_args!("non-numeric value"));
                None
            },
        }
    }
    */

    fn get_two_ints<W>(&mut self, w: &mut W) -> Option<(&BigInt, &BigInt)> where W: Write {
        let a: &BigInt;
        let b: &BigInt;

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

    fn binary_operator<W, F>(&mut self, w: &mut W, f: F)
            where W: Write,
            F: Fn(&BigInt, &BigInt) -> Result<Option<DCValue>, String> {

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

    fn binary_operator2<W, F>(&mut self, w: &mut W, f: F)
            where W: Write,
            F: Fn(&BigInt, &BigInt) -> Result<Vec<DCValue>, String> {

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

    fn pop_stack<W>(&mut self, w: &mut W) -> Option<DCValue> where W: Write {
        let value: DCValue;
        match self.stack.pop() {
            Some(v) => { value = v; },
            None    => {
                self.error(w, format_args!("stack empty"));
                return None;
            }
        }
        Some(value)
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

    pub fn program<R, W>(&mut self, r: &mut R, w: &mut W) -> DCResult
            where R: Read,
            W: Write {
        loop_over_stream(r, |c, prev| self.loop_iteration(c, prev, w) )
    }

    fn loop_iteration<W>(&mut self, c: char, prev: char, w: &mut W) -> DCResult
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

        // operations that need one more character to be read:
        let mut return_early: Option<DCResult> = Some(DCResult::Continue);
        match prev {
            's' => self.pop_stack(w).then(|value| {
                self.registers[c as usize].set(value);
            }),

            'l' => match self.registers[c as usize].value() {
                Some(value) => self.stack.push(value.clone()),
                None => self.error(w, format_args!("register '{}' (0{:o}) is empty", c, c as usize)),
            },

            'S' => self.pop_stack(w).then(|value| {
                self.registers[c as usize].push(value);
            }),

            'L' => match self.registers[c as usize].pop() {
                Some(value) => self.stack.push(value),
                None => self.error(w, format_args!("stack register '{}' (0{:o}) is empty", c, c as usize)),
            },

            _ => { return_early = None; }
        };
        match return_early {
            Some(result) => return result,
            None         => {}
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

            //println!("digit: {:?}", self.input_num.as_ref().unwrap());

            return DCResult::Continue;
        }

        if !self.input_num.is_none() {
            //println!("pushing: {:?}", self.input_num.as_ref().unwrap());
            let mut n = self.input_num.take().unwrap();
            if self.negative {
                n = n * BigInt::from(-1);
            }
            self.stack.push(DCValue::Num(n));
            self.negative = false;
        }
        else if self.negative {
            self.stack.push(DCValue::Num(BigInt::zero()));
            self.negative = false;
        }

        match c {
            ' '|'\t'|'\r'|'\n' => (), // ignore whitespace

            's'|'l'|'S'|'L' => (), // handled above

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

            'c' => self.stack.clear(),
            'd' => self.stack.last().and_then(|value| Some(value.clone())).then(|value| {
                self.stack.push(value);
            }),
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

            'I' => self.stack.push(DCValue::Num(BigInt::from(self.iradix))),
            'O' => self.stack.push(DCValue::Num(BigInt::from(self.oradix))),
            'K' => self.stack.push(DCValue::Num(BigInt::from(self.scale))),

            // pop top and execute as macro
            'x' => match self.pop_string(w).and_then(|string| {
                    let mut prev_char = '\0';
                    for c in string.chars() {
                        let return_early: Option<DCResult> = match self.loop_iteration(c, prev_char, w) {
                            DCResult::QuitLevels(n) => Some(DCResult::QuitLevels(n - 1)),
                            DCResult::Terminate => Some(DCResult::Terminate),
                            DCResult::Continue => None,
                        };
                        if return_early.is_some() {
                            return return_early;
                        }
                        prev_char = c;
                    }
                    None
                }) {
                Some(DCResult::Continue) => (),
                Some(result) => return result,
                None => ()
            },

            '+' => self.binary_operator(w, |a, b| Ok(Some(DCValue::Num(a + b)))),
            '-' => self.binary_operator(w, |a, b| Ok(Some(DCValue::Num(a - b)))),
            '*' => self.binary_operator(w, |a, b| Ok(Some(DCValue::Num(a * b)))),
            '/' => {
                self.binary_operator(w, |a, b| {
                    if b.is_zero() {
                        Err(format!("divide by zero"))
                    }
                    else {
                        Ok(Some(DCValue::Num(a / b)))
                    }
                });
            },

            // remainder
            '%' => self.binary_operator(w, |a, b| {
                if b.is_zero() {
                    Err(format!("divide by zero"))
                }
                else {
                    Ok(Some(DCValue::Num(a % b)))
                }
            }),

            // quotient and remainder
            '~' => self.binary_operator2(w, |a, b| {
                if b.is_zero() {
                    Err(format!("divide by zero"))
                }
                else {
                    let div_rem = a.div_rem(b);
                    Ok(vec![ DCValue::Num(div_rem.0), DCValue::Num(div_rem.1) ])
                }
            }),

            // exponentiate
            '^' => self.binary_operator(w, |base, exponent| {
                let mut result: BigInt;
                if exponent.is_zero() {
                    result = BigInt::one();
                }
                else if exponent.is_negative() {
                    result = BigInt::zero();
                }
                else {
                    result = base.clone();
                    for _ in range(BigInt::zero(), exponent - BigInt::one()) {
                        result = result * base;
                    }
                }
                Ok(Some(DCValue::Num(result)))
            }),

            //TODO:
            // '|': modular exponentiation
            // 'v': square root

            // catch-all for unhandled characters
            _ => self.error(w, format_args!("{:?} (0{:o}) unimplemented", c, c as u32))
        }
        DCResult::Continue
    }

    fn error<W>(&self, w: &mut W, args: Arguments) where W: Write {
        write!(w, "{}: {}\n", self.program_name, fmt::format(args)).unwrap();
    }
}
