//
// BigReal :: An arbitrary-precision real number class.
//
// Copyright (c) 2016-2018 by William R. Fraser
//

use std::cmp::{max, Ordering};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Sub, Mul, Neg, Shr};

extern crate num;
use num::BigInt;
use num::integer::Integer;
use num::traits::{Zero, One, Signed, ToPrimitive, FromPrimitive};

#[derive(Clone, Debug)]
pub struct BigReal {
    shift: u32, // in decimal digits
    value: BigInt,
}

impl BigReal {
    fn change_shift(&self, desired_shift: u32) -> BigReal {
        let mut result = self.clone();
        if desired_shift > result.shift {
            for _ in 0..(desired_shift - self.shift) {
                result.value = &result.value * 10;
            }
        }
        else {
            for _ in 0..(result.shift - desired_shift) {
                result.value = &result.value / 10;
            }
        }
        result.shift = desired_shift;
        result
    }

    /// Reduce the shift as much as possible without losing any precision.
    pub fn simplify(&mut self) {
        let ten = BigInt::from(10);
        loop {
            if self.shift == 0 {
                break;
            }
            let (quotient, remainder) = self.value.div_rem(&ten);
            if !remainder.is_zero() {
                break;
            }
            self.shift -= 1;
            self.value = quotient;
        }
    }

    pub fn set_shift(&mut self, shift: u32) {
        self.shift = shift;
    }

    pub fn to_str_radix(&self, radix: u32) -> String {
        if self.shift == 0 {
            self.value.to_str_radix(radix)
        }
        else if radix == 10 {
            // For decimal, it's fine to just put the dot in the right place.
            let mut output = if self.is_negative() {
                "-".to_string()
            } else {
                String::new()
            };

            let digits: String = self.value.abs().to_str_radix(radix);
            if digits.len() < self.shift as usize {
                // output lacks leading zeroes
                output.push('.');
                for _ in 0..(self.shift as usize - digits.len()) {
                    output.push('0');
                }
                output.push_str(&digits);
            }
            else {
                let decimal_pos = digits.len() - self.shift as usize;
                output.push_str(&digits[..decimal_pos]);
                output.push('.');
                output.push_str(&digits[decimal_pos..]);
            }
            output
        }
        else {
            // For non-decimal, the whole part is fine, but the string representation of the
            // fractional part needs to be computed manually using long division.

            let mut string_result = if self.value.is_negative() {
                "-".to_string()
            } else {
                String::new()
            };

            let whole = self.change_shift(0).abs();

            if !whole.value.is_zero() { // suppress leading zero
                string_result.push_str(&whole.value.to_str_radix(radix));
            }
            string_result.push('.');

            // start with the part shifted over one place value (because otherwise the first
            // iteration would always yield zero).
            let mut part = (&self.value - whole.change_shift(self.shift).value).abs() * radix;

            // These control when we stop the iteration.
            // When the current place value (in whatever radix) is greater than the amount of the
            // shift (in decimal), we stop.
            let max_place = BigReal::one().change_shift(self.shift).value;
            let mut place = BigInt::from(radix);

            loop {
                let div_rem = part.div_rem(&max_place);

                string_result.push_str(&div_rem.0.to_str_radix(radix));
                part = div_rem.1 * radix;

                // check if we've reached the appropriate precision
                if place >= max_place {
                    break;
                }
                place *= radix;
            }

            string_result
        }
    }

    pub fn pow(&self, exponent: &BigReal, scale: u32) -> BigReal {
        let mut negative = false;
        if exponent.is_zero() {
            return BigReal::one();
        } else if exponent.is_negative() {
            negative = true;
        }

        // Ignore the fractional part of the exponent.
        let mut exponent: BigInt = exponent.change_shift(0).value.abs();
        let one = BigInt::one();

        let mut base = self.clone();

        while exponent.is_even() {
            base = &base * &base;
            exponent = exponent.shr(1);
        }

        let mut result = base.clone();
        while (&exponent - &one).is_positive() {
            exponent = exponent.shr(1);
            base = &base * &base;
            if exponent.is_odd() {
                result = result * &base;
            }
        }

        if negative {
            BigReal::from(one).div(&result, scale)
        } else {
            result
        }
    }

    pub fn sqrt(&self, scale: u32) -> Option<BigReal> {
        if self.is_negative() {
            return None;
        }

        let scale = ::std::cmp::max(self.shift, scale);

        let mut x = self.clone();
        let one_int = BigInt::one();
        let two_real = BigReal::from(2);

        loop {
            let next = (&x + self.div(&x, scale)).div(&two_real, scale);
            let delta = (&x - &next).abs();
            x = next;

            if !(delta.value - &one_int).is_positive() {
                break;
            }
        }

        Some(x)
    }

    pub fn log2_int(&self) -> Option<BigReal> {
        if self.is_negative() || self.is_zero() {
            return None;
        }

        let (_sign, bytes) = self.value.to_bytes_le();
        let msb = bytes.last().unwrap();
        let msb_log = 8 - msb.leading_zeros() - 1;
        let result = msb_log + 8 * (bytes.len() as u32 - 1);
        Some(BigReal::from(result))
    }

    pub fn log2(&self, _scale: u32) -> Option<BigReal> {
        if self.is_negative() || self.is_zero() {
            return None;
        }

        unimplemented!("log2")
    }

    pub fn modexp(base: &BigReal, exponent: &BigReal, modulus: &BigReal, scale: u32)
            -> Option<BigReal> {
        if exponent.is_negative() || modulus.is_zero() {
            return None;
        }

        let one = BigReal::one();
        let two = BigReal::from(2);

        if (modulus - &one).is_zero() {
            return Some(BigReal::zero());
        }

        let mut base = base.rem(modulus, 0);
        let mut exponent = exponent.change_shift(0);
        let mut result = one.clone();
        while !exponent.is_zero() {
            if (exponent.rem(&two, scale) - &one).is_zero() {
                result = (result * &base).rem(modulus, 0);
            }
            exponent = exponent.div(&two, 0);
            base = (&base * &base).rem(modulus, 0);
        }

        Some(result)
    }

    pub fn is_integer(&self) -> bool {
        self.shift == 0
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

    /// Return the number as a BigInt, with the fractional part truncated off.
    pub fn to_int(&self) -> BigInt {
        let mut shifted = self.change_shift(0);
        shifted.simplify();
        assert_eq!(0, shifted.shift);
        shifted.value
    }
}

impl PartialOrd for BigReal {
    fn partial_cmp(&self, rhs: &BigReal) -> Option<Ordering> {
        if self.shift == rhs.shift {
            self.value.partial_cmp(&rhs.value)
        } else {
            let max_shift = max(self.shift, rhs.shift);
            let a = self.change_shift(max_shift);
            let b = rhs.change_shift(max_shift);
            a.value.partial_cmp(&b.value)
        }
    }
}

impl PartialEq for BigReal {
    fn eq(&self, rhs: &BigReal) -> bool {
        if self.shift == rhs.shift {
            self.value.eq(&rhs.value)
        } else {
            let max_shift = max(self.shift, rhs.shift);
            let a = self.change_shift(max_shift);
            let b = rhs.change_shift(max_shift);
            a.value.eq(&b.value)
        }
    }
}

impl Eq for BigReal {
}

impl Hash for BigReal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut simp = self.clone();
        simp.simplify();
        simp.shift.hash(state);
        simp.value.hash(state);
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
            shift,
            value,
        }
    }
}

impl From<BigInt> for BigReal {
    fn from(value: BigInt) -> BigReal {
        BigReal {
            shift: 0,
            value,
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
        let value = &self.value * &rhs.value;

        #[allow(suspicious_arithmetic_impl)]
        let shift = self.shift + rhs.shift;

        BigReal::new(value, shift)
    }
}

#[test]
fn test_new() {
    let n = BigReal::new(1234, 5);
    assert_eq!(n.value, BigInt::from(1234));
    assert_eq!(n.shift, 5);
}

#[test]
fn test_eq() {
    let a = BigReal::new(1, 2);
    let b = BigReal::new(2, 2);
    assert!(!(a == b));
    assert!(a != b);
}

#[test]
fn test_cmp() {
    let a = BigReal::new(1, 0); // 1
    let b = BigReal::new(1, 3); // .001
    assert!(a > b);
    assert!(a >= b);
    assert!(!(a < b));
    assert!(!(a <= b));
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

#[test]
fn test_simplify() {
    let a = BigReal::new(1100, 3); // 1.100
    let mut b = a.clone();
    b.simplify();
    assert!(a == b);
    assert_eq!(b.shift, 1);
    assert_eq!(b.value.to_str_radix(10), "11");
}
