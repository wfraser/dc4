//
// OptionThen: a trait providing a method 'then' for Option.
// Same as Option::and_then, except this one returns nothing.
//
// Copyright (c) 2015 by William R. Fraser
//

pub trait OptionThen<T> {
    fn then<F>(self, f: F) where F: FnOnce(T);
}

impl <T> OptionThen<T> for Option<T> {
    fn then<F>(self, f: F) where F: FnOnce(T) {
        if self.is_some() {
            f(self.unwrap());
        }
    }
}
