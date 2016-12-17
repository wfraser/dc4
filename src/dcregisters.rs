//
// dc4 registers
//
// Copyright (c) 2015-2016 by William R. Fraser
//

use std::collections::HashMap;
use std::rc::Rc;
use num::traits::Zero;
use super::big_real::BigReal;
use super::DCValue;

const MAX_REGISTER: usize = 255;

pub struct DCRegisters {
    registers: Vec<DCRegisterStack>,
}

impl DCRegisters {
    pub fn new() -> DCRegisters {
        let mut registers = Vec::with_capacity(MAX_REGISTER + 1);
        for _ in 0 .. MAX_REGISTER + 1 {
            registers.push(DCRegisterStack::new());
        }
        DCRegisters {
            registers: registers,
        }
    }

    pub fn get(&self, c: char) -> &DCRegisterStack {
        // TODO: bounds check and return an Option<> instead
        self.registers.get(c as usize).unwrap()
    }

    pub fn get_mut(&mut self, c: char) -> &mut DCRegisterStack {
        // TODO: bounds check and return an Option<> instead
        self.registers.get_mut(c as usize).unwrap()
    }
}

pub struct DCRegisterStack {
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
                None => None,
            },
            None => None,
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
                None => Rc::new(DCValue::Num(BigReal::zero()))
            },
            None => Rc::new(DCValue::Num(BigReal::zero()))
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

pub struct DCRegister {
    pub main_value: Option<DCValue>,
    pub map: HashMap<BigReal, Rc<DCValue>>,
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
