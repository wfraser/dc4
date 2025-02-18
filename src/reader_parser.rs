use std::io::{Read, Bytes};
use crate::Flavor;
use crate::parser::{Parser, Action};

pub struct ReaderParser<R: Read> {
    inner: Option<Bytes<R>>,
    parser: Parser,
    stashed: Option<u8>,
}

impl<R: Read> Iterator for ReaderParser<R> {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        let mut c = None;
        loop {
            if c.is_none() {
                c = if let Some(c) = self.stashed.take() {
                    Some(c)
                } else if let Some(mut inner) = self.inner.take() {
                    match inner.next() {
                        Some(Ok(c)) => {
                            self.inner = Some(inner); // restore inner iterator
                            Some(c)
                        }
                        Some(Err(e)) => {
                            return Some(Action::InputError(e));
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

impl<R: Read> ReaderParser<R> {
    pub fn new(input: R) -> Self {
        Self {
            inner: Some(input.bytes()),
            parser: Parser::default(),
            stashed: None,
        }
    }

    pub fn set_flavor(&mut self, flavor: Flavor) {
        self.parser.flavor = flavor;
    }
}
