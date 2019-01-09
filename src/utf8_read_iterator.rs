//
// UTF-8 Input Iterator
//
// Copyright (c) 2019 by William R. Fraser
//

use std::error::Error;
use std::fmt;
use std::io::{self, BufRead};
use std::str;

/// An iterator adapter that takes a source of bytes (a `BufRead`) and iterates over the UTF-8
/// code-points in it, preserving I/O errors and invalid UTF-8 errors
pub struct Utf8ReadIterator<R: BufRead> {
    input: R,
    buf_indices: Option<(usize, usize)>,
}

impl<R: BufRead> Utf8ReadIterator<R> {
    pub fn new(input: R) -> Self {
        Self {
            input,
            buf_indices: None,
        }
    }
}

#[derive(Debug)]
pub enum Utf8ReadError {
    Io(io::Error),
    Invalid(Vec<u8>),
}

impl Error for Utf8ReadError {
    fn description(&self) -> &str {
        "UTF-8 Read Error"
    }
}

impl fmt::Display for Utf8ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Utf8ReadError::Io(e) => write!(f, "I/O Error: {}", e),
            Utf8ReadError::Invalid(bytes) => write!(f, "Invalid UTF-8 bytes: {:x?}", bytes),
        }
    }
}

impl<R: BufRead> Iterator for Utf8ReadIterator<R> {
    type Item = Result<char, Utf8ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((start, end)) = self.buf_indices {
            if start == end {
                self.input.consume(end);
                self.buf_indices = None;
            }
        }

        let buf = match self.input.fill_buf() {
            Ok(buf) => buf,
            Err(e) => {
                return Some(Err(Utf8ReadError::Io(e)));
            }
        };

        if buf.is_empty() {
            return None;
        }

        if self.buf_indices.is_none() {
            match str::from_utf8(buf) {
                Ok(s) => {
                    self.buf_indices = Some((0, s.len()));
                }
                Err(utf8_error) => {
                    let up_to = utf8_error.valid_up_to();
                    if up_to == 0 {
                        // if up_to is 0, the error len must be present
                        let len = utf8_error.error_len().unwrap();

                        // Can't do this directly because input is still
                        // borrowed mutably:
                        //self.input.consume(len);
                        // Goofy way to force a consume next time around:
                        self.buf_indices = Some((len, len));

                        //return Some('\u{FFFD}');
                        return Some(Err(Utf8ReadError::Invalid(buf[0..len].to_owned())));
                    } else {
                        self.buf_indices = Some((0, up_to));
                    }
                }
            }
        }

        let (ref mut start, ref mut end) = self.buf_indices.as_mut().unwrap();
        let s = unsafe { str::from_utf8_unchecked(&buf[*start .. *end]) };
        let c = s.chars().next().unwrap();
        *start += c.len_utf8();

        Some(Ok(c))
    }
}
