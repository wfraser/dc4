//
// dc4 main program state
//
// Copyright (c) 2015-2021 by William R. Fraser
//

use std::fmt;
use std::io::{self, BufRead, Write};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};

use crate::big_real::BigReal;
use crate::dcregisters::DcRegisters;
use crate::parser::{Action, RegisterAction, Parser};
use crate::{DcValue, DcResult, DcError};

pub struct Dc4State {
    program_name: String,
    stack: Vec<DcValue>,
    registers: DcRegisters,
    scale: u32,
    iradix: u32,
    oradix: u32,
    current_str: Vec<u8>,
    current_num: Number,
}

impl Dc4State {
    pub fn new(program_name: String) -> Self {
        Self {
            program_name,
            stack: vec![],
            registers: DcRegisters::new(),
            scale: 0,
            iradix: 10,
            oradix: 10,
            current_str: vec![],
            current_num: Number::default(),
        }
    }

    pub fn run_macro(&mut self, mut text: Vec<u8>, w: &mut impl Write) -> DcResult {
        let mut parser = Parser::default();
        let mut tail_recursion_depth = 0;
        let mut pos = 0;
        let mut cur = None;
        let mut advance = 0;
        loop {
            if cur.is_none() {
                cur = text.get(pos).cloned();
                advance = if cur.is_some() { 1 } else { 0 };
            }

            let action = parser.step(&mut cur);
            if cur.is_none() {
                pos += advance;
            }

            match action {
                None => (),
                Some(Action::Eof) => return DcResult::Continue,
                Some(action) => {
                    let mut result = self.action(action, w);

                    while let Ok(DcResult::Macro(new_text)) = result {
                        if pos == text.len() {
                            // tail recursion! :D
                            // replace the current text with the new text and start over
                            text = new_text;
                            pos = 0;
                            cur = None;
                            advance = 0;
                            tail_recursion_depth += 1;
                            result = Ok(DcResult::Continue);
                        } else {
                            result = Ok(self.run_macro(new_text, w));
                        }
                    }

                    // the quit logic is the same for both types except for which result they return
                    macro_rules! quit_handler {
                        ($n:expr, $result_ctor:path) => {
                            if $n - 1 > tail_recursion_depth {
                                return $result_ctor($n - tail_recursion_depth - 1);
                            } else if $n - 1 == tail_recursion_depth {
                                // quitting stops here
                                return DcResult::Continue;
                            } else if $n > 0 && tail_recursion_depth > 0 {
                                // if we're doing tail recursion at all, it means our parent virtual
                                // stack frame is at the end of its text, so just unroll all the
                                // virtual frames.
                                return DcResult::Continue;
                            }
                        }
                    }

                    match result {
                        Ok(DcResult::Continue) => (),
                        Ok(DcResult::QuitLevels(n)) => quit_handler!(n, DcResult::QuitLevels),
                        Ok(DcResult::Terminate(n)) => quit_handler!(n, DcResult::Terminate),
                        Ok(DcResult::Macro(_)) => unreachable!(),
                        Err(msg) => {
                            self.error(w, format_args!("{}", msg));
                        }
                    }
                }
            }
        }
    }

    /// Convenience function for pushing a number onto the stack. Returns Err if the given string
    /// is not a valid number.
    pub fn push_number(&mut self, input: impl AsRef<[u8]>) -> Result<(), DcError> {
        let mut num = Number::default();
        let mut first = true;
        for c in input.as_ref() {
            if first && *c == b'-' {
                num.push(b'_', self.iradix)?;
            } else {
                num.push(*c, self.iradix)?;
            }
            first = false;
        }
        self.stack.push(num.finish(self.iradix));
        Ok(())
    }

    /// Convenience function for pushing a string directly onto the stack (rather than running
    /// Action::StringChar for each byte, followed by Action::PushString).
    pub fn push_string(&mut self, string: impl Into<Vec<u8>>) {
        self.stack.push(DcValue::Str(string.into()));
    }

    /// Perform the given action.
    /// Any output gets written to the given writer, as well as any warnings.
    /// Errors get returned to the caller and are not written to the writer.
    pub fn action(&mut self, action: Action, w: &mut impl Write) -> Result<DcResult, DcError> {
        match action {
            Action::NumberChar(c) => {
                self.current_num.push(c, self.iradix).expect("unexpected non-number character");
            }
            Action::PushNumber => {
                let to_push = std::mem::take(&mut self.current_num);
                self.stack.push(to_push.finish(self.iradix));
            }
            Action::StringChar(c) => {
                self.current_str.push(c);
            }
            Action::PushString => {
                self.stack.push(DcValue::Str(self.current_str.split_off(0)));
            }
            Action::Register(action, register) => match action {
                RegisterAction::Store => {
                    let value = self.pop_top()?;
                    self.registers.get_mut(register).set(value);
                }
                RegisterAction::Load => {
                    match self.registers.get(register).value() {
                        Some(value) => self.stack.push(value.clone()),
                        None => return Err(
                            format!("register '{}' (0{:o}) is empty",
                                register as char, register).into()),
                    }
                }
                RegisterAction::PushRegStack => {
                    let value = self.pop_top()?;
                    self.registers.get_mut(register).push(value);
                }
                RegisterAction::PopRegStack => {
                    match self.registers.get_mut(register).pop() {
                        Some(value) => self.stack.push(value),
                        None => return Err(
                            format!("stack register '{}' (0{:o}) is empty",
                                register as char, register).into()),
                    }
                }
                RegisterAction::Gt => return self.cond_macro(register, |a,b| b>a),
                RegisterAction::Le => return self.cond_macro(register, |a,b| b<=a),
                RegisterAction::Lt => return self.cond_macro(register, |a,b| b<a),
                RegisterAction::Ge => return self.cond_macro(register, |a,b| b>=a),
                RegisterAction::Eq => return self.cond_macro(register, |a,b| b==a),
                RegisterAction::Ne => return self.cond_macro(register, |a,b| b!=a),
                RegisterAction::StoreRegArray => {
                    let maybe_key = match self.pop_top()? {
                        DcValue::Num(n) => {
                            if n.is_negative() {
                                None
                            } else {
                                Some(n)
                            }
                        }
                        DcValue::Str(_) => None,
                    };
                    let value = self.pop_top()?;
                    match maybe_key {
                        None => return Err("array index must be a nonnegative integer".into()),
                        Some(key) => {
                            self.registers.get_mut(register).array_store(key, value);
                        }
                    }
                }
                RegisterAction::LoadRegArray => match self.pop_top()? {
                    DcValue::Num(n) if !n.is_negative() => {
                        let value = self.registers.get(register)
                            .array_load(&n)
                            .as_ref()
                            .clone();
                        self.stack.push(value);
                    }
                    _ => return Err("array index must be a nonnegative integer".into()),
                }
            }
            Action::Print => {
                match self.stack.last() {
                    Some(v) => self.print_elem(v, w),
                    None => return Err("stack empty".into())
                }
                writeln!(w).unwrap();
            }
            Action::PrintNoNewlinePop => {
                let v = self.pop_top()?;
                self.print_elem(&v, w);
                w.flush().unwrap();
            }
            Action::PrintBytesPop => {
                match self.pop_top()? {
                    DcValue::Str(s) => { w.write_all(&s).unwrap(); }
                    DcValue::Num(n) => {
                        let (_sign, bytes) = n.to_int().to_bytes_be();
                        w.write_all(&bytes).unwrap();
                    }
                }
                w.flush().unwrap();
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
                self.stack.push(DcValue::Num(n1));
                self.stack.push(DcValue::Num(n2));
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
                        match value {
                            DcValue::Num(n) => {
                                if i == 1 && n.is_negative() {
                                    return Err("negative exponent".into());
                                } else if i == 2 && n.is_zero() {
                                    return Err("remainder by zero".into());
                                }
                            },
                            DcValue::Str(_) => return Err("non-numeric value".into())
                        }
                    }
                } else {
                    return Err("stack empty".into());
                }

                let unwrap_int = |value| match value {
                    DcValue::Num(n) => n,
                    DcValue::Str(_) => unreachable!(), // already checked above
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
                self.stack.push(DcValue::Num(result));
            }
            Action::Sqrt => match self.pop_top()? {
                DcValue::Num(n) => {
                    if n.is_negative() {
                        return Err("square root of negative number".into());
                    } else if n.is_zero() {
                        self.stack.push(DcValue::Num(n));
                    } else {
                        let x = n.sqrt(self.scale).unwrap();
                        self.stack.push(DcValue::Num(x));
                    }
                }
                DcValue::Str(_) => return Err("square root of nonnumeric attempted".into()),
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
                DcValue::Num(n) if self.stack.len() >= 2 => {
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
                DcValue::Num(n) => {
                    match n.to_u32() {
                        Some(radix) if (2..=16).contains(&radix) => {
                            self.iradix = radix;
                        }
                        Some(_) | None => {
                            return Err("input base must be a number between 2 and 16 (inclusive)".into());
                        }
                    }
                }
                DcValue::Str(_) => {
                    return Err("input base must be a number between 2 and 16 (inclusive)".into());
                }
            }
            Action::SetOutputRadix => match self.pop_top()? {
                // BigInt::to_str_radix actually supports radix up to 36, but we restrict it to 16
                // here because those are the only values that will round-trip (because only
                // 'A'...'F' will be interpreted as numbers.
                // On the other hand, actual dc supports unlimited output radix, but after 16 it
                // starts to use a different format.
                DcValue::Num(n) => {
                    match n.to_u32() {
                        Some(radix) if (2..=16).contains(&radix) => {
                            self.oradix = radix;
                        }
                        Some(_) | None => {
                            return Err("output base must be a number between 2 and 16 (inclusive)".into());
                        }
                    }
                }
                DcValue::Str(_) => {
                    return Err("output base must be a number between 2 and 16 (inclusive)".into());
                }
            }
            Action::SetPrecision => match self.pop_top()? {
                DcValue::Num(n) => {
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
                DcValue::Str(_) => {
                    return Err("scale must be a nonnegative number".into());
                }
            }
            Action::LoadInputRadix => self.stack.push(DcValue::Num(BigReal::from(self.iradix))),
            Action::LoadOutputRadix => self.stack.push(DcValue::Num(BigReal::from(self.oradix))),
            Action::LoadPrecision => self.stack.push(DcValue::Num(BigReal::from(self.scale))),
            Action::Asciify => match self.pop_top()? {
                DcValue::Str(mut s) => {
                    s.truncate(1);
                    self.stack.push(DcValue::Str(s));
                }
                DcValue::Num(n) => {
                    let (_sign, bytes) = n.to_int().to_bytes_le();
                    self.stack.push(DcValue::Str(format!("{}", bytes[0] as char).into_bytes()));
                }
            }
            Action::ExecuteMacro => match self.pop_top()? {
                DcValue::Str(text) => return Ok(DcResult::Macro(text)),
                num @ DcValue::Num(_) => self.stack.push(num),
            }
            Action::Input => {
                let mut line = vec![];
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                if let Err(e) = handle.read_until(b'\n', &mut line) {
                    writeln!(w, "warning: error reading input: {}", e).unwrap();
                }
                return Ok(DcResult::Macro(line));
            }
            Action::Quit => return Ok(DcResult::Terminate(2)),
            Action::QuitLevels => match self.pop_top()? {
                DcValue::Num(n) if n.is_positive() => {
                    return n.to_u32()
                        .map(DcResult::QuitLevels)
                        .ok_or_else(|| "quit levels out of range (must fit into 32 bits)".into());
                }
                DcValue::Num(_) | DcValue::Str(_) =>
                    return Err("Q command requires a number >= 1".into()),
            }
            Action::NumDigits => match self.pop_top()? {
                DcValue::Num(n) => self.stack.push(DcValue::Num(BigReal::from(n.num_digits()))),
                DcValue::Str(s) => self.stack.push(DcValue::Num(BigReal::from(s.len()))),
            }
            Action::NumFrxDigits => match self.pop_top()? {
                DcValue::Num(n) => self.stack.push(DcValue::Num(BigReal::from(n.num_frx_digits()))),
                DcValue::Str(_) => self.stack.push(DcValue::Num(BigReal::zero())),
            }
            Action::StackDepth => {
                let depth = self.stack.len();
                self.stack.push(DcValue::Num(BigReal::from(depth)));
            }
            Action::ShellExec => {
                return Err("running shell commands is not supported".into());
            }
            Action::Version => {
                let ver = env!("CARGO_PKG_VERSION_MAJOR").parse::<u64>().unwrap() << 24
                        | env!("CARGO_PKG_VERSION_MINOR").parse::<u64>().unwrap() << 16
                        | env!("CARGO_PKG_VERSION_PATCH").parse::<u64>().unwrap();
                self.stack.push(DcValue::Num(BigReal::from(ver)));
                self.stack.push(DcValue::Str("dc4".to_owned().into_bytes()));
            }
            Action::Eof => (), // nothing to do
            Action::Unimplemented(c) => {
                return Err(format!("{:?} (0{:o}) unimplemented", c as char, c).into());
            }
            Action::InputError(msg) => {
                return Err(msg.into());
            }
        }
        Ok(DcResult::Continue)
    }

    fn print_elem(&self, elem: &DcValue, w: &mut impl Write) {
        match elem {
            DcValue::Num(n) => if n.is_zero() {
                // dc special-cases zero and ignores the scale, opting to not print the extra zero
                // digits.
                write!(w, "0")
            } else {
                write!(w, "{}", n.to_str_radix(self.oradix).to_uppercase())
            }
            DcValue::Str(s) => w.write_all(s),
        }.unwrap();
    }

    fn get_two_ints(&self) -> Result<(&BigReal, &BigReal), DcError> {
        let a: &BigReal;
        let b: &BigReal;

        let len = self.stack.len();
        if len < 2 {
            return Err("stack empty".into());
        }

        if let DcValue::Num(ref n) = self.stack[len - 2] {
            a = n;
        } else {
            return Err("non-numeric value".into());
        }

        if let DcValue::Num(ref n) = self.stack[len - 1] {
            b = n;
        } else {
            return Err("non-numeric value".into());
        }

        Ok((a, b))
    }

    fn pop_top(&mut self) -> Result<DcValue, DcError> {
        self.stack.pop()
            .ok_or_else(|| "stack empty".into())
    }

    fn binary_lambda<T, F>(&mut self, mut f: F) -> Result<T, DcError>
        where F: FnMut(&BigReal, &BigReal) -> Result<T, DcError>
    {
        let value: T = {
            let (a, b) = self.get_two_ints()?;
            f(a, b)?
        };

        self.stack.pop();
        self.stack.pop();
        Ok(value)
    }

    fn binary_operator<F>(&mut self, mut f: F) -> Result<(), DcError>
        where F: FnMut(&BigReal, &BigReal) -> Result<BigReal, DcError>
    {
        let n = self.binary_lambda(|a, b| f(a, b))?;
        self.stack.push(DcValue::Num(n));
        Ok(())
    }

    fn cond_macro<F>(&mut self, register: u8, f: F) -> Result<DcResult, DcError>
        where F: Fn(&BigReal, &BigReal) -> bool
    {
        if self.binary_lambda(|a, b| Ok(f(a, b)))? {
            let text = match self.registers.get(register).value() {
                Some(DcValue::Str(s)) => s.to_owned(),
                Some(DcValue::Num(_)) => return Ok(DcResult::Continue),
                None => return Err(
                    format!("register '{}' (0{:o}) is empty", register as char, register).into()),
            };
            Ok(DcResult::Macro(text))
        } else {
            Ok(DcResult::Continue)
        }
    }

    pub(crate) fn error(&self, w: &mut impl Write, args: fmt::Arguments<'_>) {
        writeln!(w, "{}: {}", self.program_name, fmt::format(args)).unwrap();
    }
}

// A number in the process of being built up from input.
#[derive(Default)]
struct Number {
    int: BigInt,
    shift: Option<u32>,
    neg: bool,
}

impl Number {
    pub fn push(&mut self, c: u8, iradix: u32) -> Result<(), String> {
        match c {
            b'_' => { self.neg = true; }
            b'0' ..= b'9' | b'A' ..= b'F' => {
                self.int *= iradix;
                self.int += (c as char).to_digit(16).unwrap();
                if let Some(shift) = self.shift.as_mut() {
                    *shift += 1;
                }
            }
            b'.' => { self.shift = Some(0); }
            _ => return Err(format!("unexpected character in number: {:?}", c as char)),
        }
        Ok(())
    }

    pub fn finish(mut self, iradix: u32) -> DcValue {
        if self.neg {
            self.int *= -1;
        }
        let mut real = BigReal::from(self.int);
        if let Some(shift) = self.shift {
            if iradix == 10 {
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
                let divisor = BigReal::from(iradix);
                for _ in 0 .. shift {
                    real = real.div(&divisor, shift);
                }
            }
        }
        DcValue::Num(real)
    }
}
