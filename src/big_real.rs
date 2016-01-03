//
// BigReal :: An arbitrary-precision real number class.
//
// Copyright (c) 2016 by William R. Fraser
//

use std::ops::{Add, Sub, Mul, Neg};

extern crate num;
use num::{BigInt};

#[derive(Clone, Debug)]
pub struct BigReal {
    scale: u32, // decimal digits to shift
    value: BigInt,
}

pub trait BigRealFrom<T>: Sized {
    fn new(value: T, scale: u32) -> Self;
}

impl BigReal {
    pub fn change_scale(&mut self, desired_scale: u32) {
        let ten = BigInt::from(10);
        if desired_scale > self.scale {
            for _ in 0..(desired_scale - self.scale) {
                self.value = &self.value * &ten;
            }
        }
        else {
            for _ in 0..(self.scale - desired_scale) {
                self.value = &self.value / &ten;
            }
        }
        self.scale = desired_scale;
    }
}

macro_rules! bigreal_from_primitive {
    ($prim:ident) => {
        impl BigRealFrom<$prim> for BigReal {
            fn new(value: $prim, scale: u32) -> BigReal {
                BigReal::new(BigInt::from(value), scale)
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
    fn new(value: BigInt, scale: u32) -> BigReal {
        BigReal {
            scale: scale,
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
        if self.scale == rhs.scale {
            BigReal::new(&self.value + &rhs.value, self.scale)
        }
        else {
            let x: &BigReal;
            let mut y: BigReal;
            if self.scale > rhs.scale {
                // adjust rhs
                x = self;
                y = (*rhs).clone();
            }
            else {
                // adjust self
                x = rhs;
                y = (*self).clone();
            }
            y.change_scale(x.scale);
            BigReal::new(&x.value + y.value, x.scale)
        }
    }
}

forward_all_binop_to_ref_ref!(impl Sub for BigReal, sub);

impl<'a, 'b> Sub<&'b BigReal> for &'a BigReal {
    type Output = BigReal;

    #[inline]
    fn sub(self, rhs: &BigReal) -> BigReal {
        self.add(BigReal::new(rhs.value.clone().neg(), rhs.scale))
    }
}

forward_all_binop_to_ref_ref!(impl Mul for BigReal, mul);

impl<'a, 'b> Mul<&'b BigReal> for &'a BigReal {
    type Output = BigReal;

    fn mul(self, rhs: &BigReal) -> BigReal {
        BigReal::new(&self.value * &rhs.value, self.scale + rhs.scale)
    }
}

#[test]
fn test_add() {
    let a = BigReal::new(1234, 3);
    let b = BigReal::new(42, 0);
    let c = a + b;
    assert_eq!(c.value, BigInt::from(43234));
    assert_eq!(c.scale, 3);
}

#[test]
fn test_sub() {
    let a = BigReal::new(1234, 3);
    let b = BigReal::new(42, 0);
    let c = a - b;
    assert_eq!(c.value, BigInt::from(-40766));
    assert_eq!(c.scale, 3);
}

#[test]
fn test_mul1() {
    let a = BigReal::new(25, 0);
    let b = BigReal::new(4, 0);
    let c = a * b;
    assert_eq!(c.value, BigInt::from(100));
    assert_eq!(c.scale, 0);
}

#[test]
fn test_mul2() {
    let a = BigReal::new(25, 1);
    let b = BigReal::new(4, 2);
    let c = a * b;
    assert_eq!(c.value, BigInt::from(100));
    assert_eq!(c.scale, 3);
}
