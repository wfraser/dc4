use std::io::BufRead;
use parser::{Parser, Action};
use utf8_read_iterator::{Utf8ReadIterator, Utf8ReadError};

pub struct ByteActionParser<R: BufRead> {
    inner: Option<Utf8ReadIterator<R>>,
    parser: Parser,
    stashed: Option<char>,
}

impl<R: BufRead> Iterator for ByteActionParser<R> {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        let mut c = None;
        loop {
            if c.is_none() {
                c = if let Some(c) = self.stashed.take() {
                    Some(c)
                } else if let Some(mut inner) = self.inner.take() {
                    match inner.next() {
                        Some(Err(Utf8ReadError::Io(e))) => {
                            return Some(Action::InputError(format!("I/O error reading input: {}", e)));
                        }
                        Some(Err(Utf8ReadError::Invalid(bytes))) => {
                            self.stashed = Some('\u{FFFD}');
                            self.inner = Some(inner);
                            return Some(Action::InputError(format!("Invalid UTF-8 in input: {:x?}", bytes)));
                        }
                        Some(Ok(c)) => {
                            self.inner = Some(inner); // restore inner iterator
                            Some(c)
                        }
                        None => None,
                    }
                } else {
                    None
                };
            }

            if let Some(action) = self.parser.step(&mut c) {
                if let Some(unused_char) = c {
                    // if the parser didn't use the character, stash it for next time around.
                    self.stashed = Some(unused_char);
                }
                if let Action::Eof = action {
                    self.inner = None;
                    return None;
                } else {
                    return Some(action);
                }
            }
        }
    }
}

impl<R: BufRead> ByteActionParser<R> {
    pub fn new(input: R) -> Self {
        Self {
            inner: Some(Utf8ReadIterator::new(input)),
            parser: Parser::new(),
            stashed: None,
        }
    }
}
