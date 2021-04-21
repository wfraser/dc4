//
// dc4 registers
//
// Copyright (c) 2015-2021 by William R. Fraser
//

use std::collections::HashMap;
use std::rc::Rc;
use num_traits::Zero;
use crate::big_real::BigReal;
use crate::DcValue;

const MAX_REGISTER: usize = 255;

pub struct DcRegisters {
    registers: Vec<DcRegisterStack>,
}

impl DcRegisters {
    pub fn new() -> DcRegisters {
        let mut registers = Vec::with_capacity(MAX_REGISTER + 1);
        for _ in 0 ..= MAX_REGISTER {
            registers.push(DcRegisterStack::new());
        }
        DcRegisters {
            registers,
        }
    }

    pub fn get(&self, c: u8) -> &DcRegisterStack {
        &self.registers[c as usize]
    }

    pub fn get_mut(&mut self, c: u8) -> &mut DcRegisterStack {
        &mut self.registers[c as usize]
    }
}

pub struct DcRegisterStack {
    stack: Vec<DcRegister>,
}

impl DcRegisterStack {
    pub fn new() -> DcRegisterStack {
        DcRegisterStack {
            stack: Vec::new()
        }
    }

    pub fn value(&self) -> Option<&DcValue> {
        match self.stack.last() {
            Some(reg) => reg.main_value.as_ref(),
            None => None,
        }
    }

    pub fn array_store(&mut self, key: BigReal, value: DcValue) {
        if self.stack.is_empty() {
            self.stack.push(DcRegister::new(None));
        }
        self.stack.last_mut().unwrap().map_insert(key, value);
    }

    pub fn array_load(&self, key: &BigReal) -> Rc<DcValue> {
        match self.stack.last() {
            Some(reg) => match reg.map_lookup(key) {
                Some(value) => value.clone(),
                None => Rc::new(DcValue::Num(BigReal::zero()))
            },
            None => Rc::new(DcValue::Num(BigReal::zero()))
        }
    }

    pub fn set(&mut self, value: DcValue) {
        if !self.stack.is_empty() {
            self.stack.pop();
        }
        self.stack.push(DcRegister::new(Some(value)));
    }

    pub fn pop(&mut self) -> Option<DcValue> {
        self.stack.pop().and_then(|v| v.main_value)
    }

    pub fn push(&mut self, value: DcValue) {
        self.stack.push(DcRegister::new(Some(value)))
    }
}

pub struct DcRegister {
    pub main_value: Option<DcValue>,
    pub map: HashMap<BigReal, Rc<DcValue>>,
}

impl DcRegister {
    pub fn new(value: Option<DcValue>) -> DcRegister {
        DcRegister {
            main_value: value,
            map: HashMap::new(),
        }
    }

    pub fn map_lookup(&self, key: &BigReal) -> Option<&Rc<DcValue>> {
        self.map.get(key)
    }

    pub fn map_insert(&mut self, key: BigReal, value: DcValue) {
        self.map.insert(key, Rc::new(value));
    }
}
