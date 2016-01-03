//
// BigReal :: An arbitrary-precision real number class.
//
// Copyright (c) 2016 by William R. Fraser
//

use std::ops::{Add, Sub, Mul, Neg};

extern crate num;
use num::{BigInt};

#[derive(Clone, Debug, PartialEq)]
pub struct BigReal {
    shift: u32, // in decimal digits
    value: BigInt,
}

pub trait BigRealFrom<T>: Sized {
    fn new(value: T, shift: u32) -> Self;
}

impl BigReal {
    pub fn change_shift(&mut self, desired_shift: u32) {
        let ten = BigInt::from(10);
        if desired_shift > self.shift {
            for _ in 0..(desired_shift - self.shift) {
                self.value = &self.value * &ten;
            }
        }
        else {
            for _ in 0..(self.shift - desired_shift) {
                self.value = &self.value / &ten;
            }
        }
        self.shift = desired_shift;
    }
}

macro_rules! bigreal_from_primitive {
    ($prim:ident) => {
        impl BigRealFrom<$prim> for BigReal {
            fn new(value: $prim, shift: u32) -> BigReal {
                BigReal::new(BigInt::from(value), shift)
            }
        }
    }
}

bigreal_from_primitive!(u8);
bigreal_from_primitive!(u16);
bigreal_from_primitive!(u32);
bigreal_from_primitive!(u64);
bigreal_from_primitive!(usize);
bigreal_from_primitive!(i8);
bigreal_from_primitive!(i16);
bigreal_from_primitive!(i32);
bigreal_from_primitive!(i64);
bigreal_from_primitive!(isize);

impl BigRealFrom<BigInt> for BigReal {
    fn new(value: BigInt, shift: u32) -> BigReal {
        BigReal {
            shift: shift,
            value: value,
        }
    }
}

macro_rules! forward_val_val_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        impl $imp<$res> for $res {
            type Output = $res;

            #[inline]
            fn $method(self, rhs: $res) -> $res {
                (&self).$method(&rhs)
            }
        }
    }
}

macro_rules! forward_val_ref_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        impl<'a> $imp<&'a $res> for $res {
            type Output = $res;

            #[inline]
            fn $method(self, rhs: &$res) -> $res {
                (&self).$method(rhs)
            }
        }
    }
}

macro_rules! forward_ref_val_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        impl<'a> $imp<$res> for &'a $res {
            type Output = $res;

            #[inline]
            fn $method(self, rhs: $res) -> $res {
                self.$method(&rhs)
            }
        }
    }
}

macro_rules! forward_all_binop_to_ref_ref {
    (impl $imp:ident for $res:ty, $method:ident) => {
        forward_val_val_binop!(impl $imp for $res, $method);
        forward_val_ref_binop!(impl $imp for $res, $method);
        forward_ref_val_binop!(impl $imp for $res, $method);
    }
}

forward_all_binop_to_ref_ref!(impl Add for BigReal, add);

impl<'a, 'b> Add<&'b BigReal> for &'a BigReal {
    type Output = BigReal;

    fn add(self, rhs: &BigReal) -> BigReal {
        if self.shift == rhs.shift {
            BigReal::new(&self.value + &rhs.value, self.shift)
        }
        else {
            let x: &BigReal;
            let mut y: BigReal;
            if self.shift > rhs.shift {
                // adjust rhs
                x = self;
                y = (*rhs).clone();
            }
            else {
                // adjust self
                x = rhs;
                y = (*self).clone();
            }
            y.change_shift(x.shift);
            BigReal::new(&x.value + y.value, x.shift)
        }
    }
}

forward_all_binop_to_ref_ref!(impl Sub for BigReal, sub);

impl<'a, 'b> Sub<&'b BigReal> for &'a BigReal {
    type Output = BigReal;

    #[inline]
    fn sub(self, rhs: &BigReal) -> BigReal {
        self.add(BigReal::new(rhs.value.clone().neg(), rhs.shift))
    }
}

forward_all_binop_to_ref_ref!(impl Mul for BigReal, mul);

impl<'a, 'b> Mul<&'b BigReal> for &'a BigReal {
    type Output = BigReal;

    fn mul(self, rhs: &BigReal) -> BigReal {
        BigReal::new(&self.value * &rhs.value, self.shift + rhs.shift)
    }
}

#[test]
fn test_new() {
    let n = BigReal::new(1234, 5);
    assert_eq!(n.value, BigInt::from(1234));
    assert_eq!(n.shift, 5);
}

#[test]
fn test_cmp() {
    let a = BigReal::new(1, 2);
    let b = BigReal::new(2, 2);
    assert!(a != b);
}

#[test]
fn test_add() {
    let a = BigReal::new(1234, 3);
    let b = BigReal::new(42, 0);
    let c = a + b;
    assert_eq!(c, BigReal::new(43234, 3));
}

#[test]
fn test_sub() {
    let a = BigReal::new(1234, 3);
    let b = BigReal::new(42, 0);
    let c = a - b;
    assert_eq!(c, BigReal::new(-40766, 3));
}

#[test]
fn test_mul1() {
    let a = BigReal::new(25, 0);
    let b = BigReal::new(4, 0);
    let c = a * b;
    assert_eq!(c, BigReal::new(100, 0));
}

#[test]
fn test_mul2() {
    let a = BigReal::new(25, 1);
    let b = BigReal::new(4, 2);
    let c = a * b;
    assert_eq!(c, BigReal::new(100, 3));
}
