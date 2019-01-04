//
// dc4 main program state
//
// Copyright (c) 2015-2019 by William R. Fraser
//

use std::fmt;
use std::io::{self, BufRead, Write};
use num::BigInt;
use num::traits::{ToPrimitive, Zero};

use big_real::BigReal;
use byte_parser::ByteActionParser;
use dcregisters::DCRegisters;
use parser::{Action, RegisterAction};
use ::{DCValue, DCResult, DCError};

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

    pub fn program(&mut self, r: &mut impl BufRead, w: &mut impl Write) -> DCResult {
        for action in ByteActionParser::new(r) {
            match self.action(action, w) {
                Ok(DCResult::Continue) => (), // next loop iteration
                Ok(DCResult::Macro(text)) => unimplemented!("macro {:?}", text),
                Err(msg) => {
                    self.error(w, format_args!("{}", msg));
                }
                Ok(other) => {
                    return other;
                }
            }
        }
        DCResult::Continue
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
                let v = self.pop_top()?;
                self.print_elem(&v, w);
            }
            Action::PrintBytesPop => match self.pop_top()? {
                DCValue::Str(s) => { write!(w, "{}", s).unwrap(); },
                DCValue::Num(n) => {
                    let (_sign, bytes) = n.to_int().to_bytes_be();
                    w.write_all(&bytes).unwrap();
                }
            }
            Action::PrintStack => {
                for value in self.stack.iter().rev() {
                    self.print_elem(value, w);
                    writeln!(w).unwrap();
                }
            }
            Action::Add => self.binary_operator(|a, b| Ok(a + b))?,
            Action::Sub => self.binary_operator(|a, b| Ok(a - b))?,
            Action::Mul => self.binary_operator(|a, b| Ok(a * b))?,
            Action::Div => {
                let scale = self.scale;
                self.binary_operator(|a, b| {
                    if b.is_zero() {
                        Err("divide by zero".into())
                    } else {
                        Ok(a.div(b, scale))
                    }
                })?
            }
            Action::Rem => {
                let scale = self.scale;
                self.binary_operator(|a, b| {
                    if b.is_zero() {
                        Err("remainder by zero".into())
                    } else {
                        Ok(a.rem(b, scale))
                    }
                })?
            }
            Action::DivRem => {
                let scale = self.scale;
                let (n1, n2) = {
                    let (a, b) = self.get_two_ints()?;
                    if b.is_zero() {
                        return Err("divide by zero".into());
                    }
                    a.div_rem(b, scale)
                };
                self.stack.pop();
                self.stack.pop();
                self.stack.push(DCValue::Num(n1));
                self.stack.push(DCValue::Num(n2));
            }
            Action::Exp => {
                let mut warn = false;
                let scale = self.scale;
                self.binary_operator(|base, exponent| {
                    if !exponent.is_integer() {
                        // have to print the warning outside the closure
                        warn = true;
                    }

                    Ok(base.pow(exponent, scale))
                })?;
                if warn {
                    // note: GNU dc doesn't emit any warning here.
                    self.error(w, format_args!("warning: non-zero scale in exponent"));
                }
            }
            Action::ModExp => {
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

                let unwrap_int = |value| match value {
                    DCValue::Num(n) => n,
                    _ => unreachable!(), // already checked above
                };
                let modulus = self.stack.pop().map(unwrap_int).unwrap();
                let exponent = self.stack.pop().map(unwrap_int).unwrap();
                let base = self.stack.pop().map(unwrap_int).unwrap();

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
            }
            Action::Sqrt => match self.pop_top()? {
                DCValue::Num(n) => {
                    if n.is_negative() {
                        return Err("square root of negative number".into());
                    } else if n.is_zero() {
                        self.stack.push(DCValue::Num(n));
                    } else {
                        let x = n.sqrt(self.scale).unwrap();
                        self.stack.push(DCValue::Num(x));
                    }
                }
                DCValue::Str(_) => return Err("square root of nonnumeric attempted".into()),
            }
            Action::ClearStack => self.stack.clear(),
            Action::Dup => if let Some(value) = self.stack.last().cloned() {
                self.stack.push(value);
            }
            Action::Swap => {
                if self.stack.len() >= 2 {
                    let a = self.stack.len() - 1;
                    let b = self.stack.len() - 2;
                    self.stack.swap(a, b);
                } else {
                    return Err("stack empty".into());
                }
            }
            Action::Rotate => match self.pop_top()? {
                DCValue::Num(ref n) if self.stack.len() >= 2 => {
                    let n = match n.to_i32() {
                        Some(n) => n,
                        None => {
                            return Err("rotation value must fit in 32 bits".into());
                        }
                    };

                    let start = match n.abs() as usize {
                        0 | 1                       => self.stack.len() - 1,
                        n if n >= self.stack.len()  => 0,
                        other                       => self.stack.len() - other,
                    };

                    if n > 0 {
                        self.stack[start..].rotate_left(1);
                    } else {
                        self.stack[start..].rotate_right(1);
                    }
                }
                _ => (), // do nothing, even if it's the wrong type!
            }
            Action::SetInputRadix => match self.pop_top()? {
                DCValue::Num(n) => {
                    match n.to_u32() {
                        Some(radix) if radix >= 2 && radix <= 16 => {
                            self.iradix = radix;
                        }
                        Some(_) | None => {
                            return Err("input base must be a number between 2 and 16 (inclusive)".into());
                        }
                    }
                }
                DCValue::Str(_) => {
                    return Err("input base must be a number between 2 and 16 (inclusive)".into());
                }
            }
            Action::SetOutputRadix => match self.pop_top()? {
                // BigInt::to_str_radix actually supports radix up to 36, but we restrict it to 16
                // here because those are the only values that will round-trip (because only
                // 'A'...'F' will be interpreted as numbers.
                // On the other hand, actual dc supports unlimited output radix, but after 16 it
                // starts to use a different format.
                DCValue::Num(n) => {
                    match n.to_u32() {
                        Some(radix) if radix >= 2 && radix <= 16 => {
                            self.oradix = radix;
                        }
                        Some(_) | None => {
                            return Err("output base must be a number between 2 and 16 (inclusive)".into());
                        }
                    }
                }
                DCValue::Str(_) => {
                    return Err("output base must be a number between 2 and 16 (inclusive)".into());
                }
            }
            Action::SetPrecision => match self.pop_top()? {
                DCValue::Num(n) => {
                    if n.is_negative() {
                        return Err("scale must be a nonnegative number".into());
                    }
                    match n.to_u32() {
                        Some(scale) => {
                            self.scale = scale;
                        }
                        None => {
                            return Err("scale must fit into 32 bits".into());
                        }
                    }
                }
                DCValue::Str(_) => {
                    return Err("scale must be a nonnegative number".into());
                }
            }
            Action::LoadInputRadix => self.stack.push(DCValue::Num(BigReal::from(self.iradix))),
            Action::LoadOutputRadix => self.stack.push(DCValue::Num(BigReal::from(self.oradix))),
            Action::LoadPrecision => self.stack.push(DCValue::Num(BigReal::from(self.scale))),
            Action::Asciify => match self.pop_top()? {
                DCValue::Str(mut s) => {
                    if let Some((len, _char)) = s.char_indices().nth(1) {
                        s.truncate(len);
                    }
                    self.stack.push(DCValue::Str(s));
                }
                DCValue::Num(n) => {
                    let (_sign, bytes) = n.to_int().to_bytes_le();
                    self.stack.push(DCValue::Str(format!("{}", bytes[0] as char)));
                }
            }
            Action::ExecuteMacro => if let DCValue::Str(string) = self.pop_top()? {
                return Ok(DCResult::Macro(string));
            }
            Action::Input => {
                let mut line = String::new();
                if let Err(e) = io::stdin().read_line(&mut line) {
                    writeln!(w, "warning: error reading input: {}", e).unwrap();
                }
                return Ok(DCResult::Macro(line));
            }
            Action::Quit => return Ok(DCResult::Terminate(2)),
            Action::QuitLevels => match self.pop_top()? {
                DCValue::Num(ref n) if n.is_positive() => {
                    return n.to_u32()
                        .map(DCResult::QuitLevels)
                        .ok_or_else(|| "quit levels out of range (must fit into 32 bits)".into());
                }
                _ => {
                    return Err("Q command requires a number >= 1".into());
                }
            }
            Action::NumDigits => match self.pop_top()? {
                DCValue::Num(n) => self.stack.push(DCValue::Num(BigReal::from(n.num_digits()))),
                DCValue::Str(s) => self.stack.push(DCValue::Num(BigReal::from(s.len()))),
            }
            Action::NumFrxDigits => match self.pop_top()? {
                DCValue::Num(n) => self.stack.push(DCValue::Num(BigReal::from(n.num_frx_digits()))),
                DCValue::Str(_) => self.stack.push(DCValue::Num(BigReal::zero())),
            }
            Action::StackDepth => {
                let depth = self.stack.len();
                self.stack.push(DCValue::Num(BigReal::from(depth)));
            }
            Action::ShellExec(_) => {
                return Err("running shell commands is not supported".into());
            }
            Action::Version => {
                let ver = env!("CARGO_PKG_VERSION_MAJOR").parse::<u64>().unwrap() << 24
                        | env!("CARGO_PKG_VERSION_MINOR").parse::<u64>().unwrap() << 16
                        | env!("CARGO_PKG_VERSION_PATCH").parse::<u64>().unwrap();
                self.stack.push(DCValue::Num(BigReal::from(ver)));
                self.stack.push(DCValue::Str("dc4".to_owned()));
            }
            Action::Eof => (), // nothing to do
            Action::Unimplemented(c) => {
                return Err(format!("{:?} (0{:o}) unimplemented", c, c as u32).into());
            }
            Action::InputError(msg) => {
                return Err(format!("error reading input: {}", msg).into());
            }
        }
        Ok(DCResult::Continue)
    }

    fn print_elem(&self, elem: &DCValue, w: &mut impl Write) {
        match *elem {
            DCValue::Num(ref n) => write!(w, "{}", n.to_str_radix(self.oradix).to_uppercase()),
            DCValue::Str(ref s) => write!(w, "{}", s),
        }.unwrap();
    }

    fn get_two_ints(&self) -> Result<(&BigReal, &BigReal), DCError> {
        let a: &BigReal;
        let b: &BigReal;

        let len = self.stack.len();
        if len < 2 {
            return Err("stack empty".into());
        }

        if let DCValue::Num(ref n) = self.stack[len - 2] {
            a = n;
        } else {
            return Err("non-numeric value".into());
        }

        if let DCValue::Num(ref n) = self.stack[len - 1] {
            b = n;
        } else {
            return Err("non-numeric value".into());
        }

        Ok((a, b))
    }

    fn pop_top(&mut self) -> Result<DCValue, DCError> {
        self.stack.pop()
            .ok_or_else(|| "stack empty".into())
    }

    fn binary_operator<F>(&mut self, mut f: F) -> Result<(), DCError>
        where F: FnMut(&BigReal, &BigReal) -> Result<BigReal, DCError>
    {
        let n: BigReal = {
            let (a, b) = self.get_two_ints()?;
            f(a, b)?
        };
        self.stack.pop();
        self.stack.pop();
        self.stack.push(DCValue::Num(n));
        Ok(())
    }

    fn error(&self, w: &mut impl Write, args: fmt::Arguments) {
        writeln!(w, "{}: {}", self.program_name, fmt::format(args)).unwrap();
    }
}
