//
// BigReal :: An arbitrary-precision real number class.
//
// Copyright (c) 2016 by William R. Fraser
//

use std::cmp::max;
use std::ops::{Add, Sub, Mul, Neg};

extern crate num;
use num::BigInt;
use num::integer::Integer;
use num::traits::{Zero, One, Signed, ToPrimitive, FromPrimitive};

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Hash)]
pub struct BigReal {
    shift: u32, // in decimal digits
    value: BigInt,
}

impl BigReal {
    pub fn change_shift(&self, desired_shift: u32) -> BigReal {
        let ten = BigInt::from(10);
        let mut result = self.clone();
        if desired_shift > result.shift {
            for _ in 0..(desired_shift - self.shift) {
                result.value = &result.value * &ten;
            }
        }
        else {
            for _ in 0..(result.shift - desired_shift) {
                result.value = &result.value / &ten;
            }
        }
        result.shift = desired_shift;
        result
    }

    pub fn set_shift(&mut self, shift: u32) {
        self.shift = shift;
    }

    pub fn get_shift(&self) -> u32 {
        self.shift
    }

    pub fn to_str_radix(&self, radix: u32) -> String {
        if self.shift == 0 {
            self.value.to_str_radix(radix)
        }
        else if radix == 10 {
            // For decimal, it's fine to just put the dot in the right place.
            let output: String = self.value.to_str_radix(radix);
            if output.len() < self.shift as usize {
                // output lacks leading zeroes
                let mut s = ".".to_string();
                for _ in 0..(self.shift as usize - output.len()) {
                    s.push_str("0");
                }
                s + &output
            }
            else {
                let decimal_pos = output.len() - self.shift as usize;
                output[..decimal_pos].to_string() + "." + &output[decimal_pos..]
            }
        }
        else {
            // For non-decimal, the whole part is fine, but the string representation of the
            // fractional part needs to be computed manually using long division.

            let whole = self.change_shift(0);
            let mut string_result = if whole.value.is_zero() {
                // suppress leading zero
                ".".to_string()
            }
            else {
                whole.value.to_str_radix(radix) + "."
            };

            let big_radix = BigInt::from(radix);

            // start with the part shifted over one place value (because otherwise the first
            // iteration would always yield zero).
            let mut part = (&self.value - whole.change_shift(self.shift).value) * &big_radix;

            // These control when we stop the iteration.
            // When the current place value (in whatever radix) is greater than the amount of the
            // shift (in decimal), we stop.
            let max_place = BigReal::one().change_shift(self.shift).value;
            let mut place = big_radix.clone();

            loop {
                let div_rem = part.div_rem(&max_place);

                string_result.push_str(&div_rem.0.to_str_radix(radix));
                part = div_rem.1 * &big_radix;

                // check if we've reached the appropriate precision
                if place >= max_place {
                    break;
                }
                place = place * &big_radix;
            }

            string_result
        }
    }

    // Our own implementations of Div and Rem, which need an extra "scale" argument:

    fn adjust_for_div(&self, rhs: &BigReal, scale: u32) -> (BigInt, BigInt) {
        let max_shift = max(self.shift, rhs.shift);
        let self_adj = self.change_shift(max_shift + scale).value;
        let rhs_adj = rhs.change_shift(max_shift).value;
        (self_adj, rhs_adj)
    }

    pub fn div(&self, rhs: &BigReal, scale: u32) -> BigReal {
        let (self_adj, rhs_adj) = self.adjust_for_div(rhs, scale);
        BigReal::new(self_adj / rhs_adj, scale)
    }

    pub fn rem(&self, rhs: &BigReal, scale: u32) -> BigReal {
        let (self_adj, rhs_adj) = self.adjust_for_div(rhs, scale);
        BigReal::new(self_adj % rhs_adj, scale)
    }

    pub fn div_rem(&self, rhs: &BigReal, scale: u32) -> (BigReal, BigReal) {
        let (self_adj, rhs_adj) = self.adjust_for_div(rhs, scale);
        let div_rem = self_adj.div_rem(&rhs_adj);
        (BigReal::new(div_rem.0, scale), BigReal::new(div_rem.1, scale))
    }

    // These are in num::traits::Signed, but that requires num::traits::Num, which we don't want to
    // implement fully, because it requires Div and Rem (which we can't implement because those
    // methods need an extra "scale" argument here).
    pub fn is_positive(&self) -> bool {
        self.value.is_positive()
    }

    pub fn is_negative(&self) -> bool {
        self.value.is_negative()
    }

    pub fn abs(&self) -> BigReal {
        BigReal::new(self.value.abs(), self.shift)
    }
}

impl Zero for BigReal {
    fn zero() -> BigReal {
        BigReal::from(0)
    }

    fn is_zero(&self) -> bool {
        self.value.is_zero()
    }
}

impl One for BigReal {
    fn one() -> BigReal {
        BigReal::from(1)
    }
}

pub trait BigRealFrom<T>: Sized {
    fn new(value: T, shift: u32) -> Self;
}

macro_rules! bigreal_from_primitive {
    ($prim:ident) => {
        impl BigRealFrom<$prim> for BigReal {
            fn new(value: $prim, shift: u32) -> BigReal {
                BigReal::new(BigInt::from(value), shift)
            }
        }

        impl From<$prim> for BigReal {
            fn from(value: $prim) -> BigReal {
                BigReal::new(BigInt::from(value), 0)
            }
        }
    }
}

impl ToPrimitive for BigReal {
    fn to_i64(&self) -> Option<i64> {
        self.change_shift(0).value.to_i64()
    }

    fn to_u64(&self) -> Option<u64> {
        self.change_shift(0).value.to_u64()
    }
}

impl FromPrimitive for BigReal {
    fn from_i64(n: i64) -> Option<BigReal> {
        Some(BigReal::from(n))
    }

    fn from_u64(n: u64) -> Option<BigReal> {
        Some(BigReal::from(n))
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

impl From<BigInt> for BigReal {
    fn from(value: BigInt) -> BigReal {
        BigReal {
            shift: 0,
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
            let (x, y): (&BigReal, &BigReal) = if self.shift > rhs.shift {
                // adjust rhs
                (self, rhs)
            } else {
                // adjust self
                (rhs, self)
            };
            BigReal::new(&x.value + y.change_shift(x.shift).value, x.shift)
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

#[test]
fn test_div1() {
    let a = BigReal::new(50, 0);    // 50.
    let b = BigReal::new(55, 3);    //  0.055
    let c = a.div(&b, 0);
    assert_eq!(c, BigReal::new(909, 0));
}

#[test]
fn test_div2() {
    let a = BigReal::new(505, 1);   // 50.5
    let b = BigReal::new(55, 3);    //  0.055
    let c = a.div(&b, 1);
    assert_eq!(c, BigReal::new(9181, 1));
}

#[test]
fn test_str1() {
    let a = BigReal::new(1234, 3);  // 1.234
    assert_eq!(a.to_str_radix(10), "1.234");
    assert_eq!(a.to_str_radix(16), "1.3be");
    assert_eq!(a.to_str_radix(2), "1.0011101111");
}

#[test]
fn test_str2() {
    let a = BigReal::new(1100, 3); // 1.100
    assert_eq!(a.to_str_radix(10), "1.100");
    assert_eq!(a.to_str_radix(16), "1.199");
    assert_eq!(a.to_str_radix(2), "1.0001100110");
}
