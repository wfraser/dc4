//
// dc4 input parser
//
// Copyright (c) 2019 by William R. Fraser
//

pub struct Parser {
    state: Option<ParseState>,
}

#[derive(Debug)]
pub enum Action {
    PushNumber(String),
    PushString(String),

    Register(RegisterAction, u8),

    Print,              // 'p'
    PrintNoNewlinePop,  // 'n'
    PrintBytesPop,      // 'P'
    PrintStack,         // 'f'

    Add,                // '+'
    Sub,                // '-'
    Mul,                // '*'
    Div,                // '/'
    Rem,                // '%'
    DivRem,             // '~'
    Exp,                // '^'
    ModExp,             // '|'
    Sqrt,               // 'v'

    ClearStack,         // 'c'
    Dup,                // 'd'
    Swap,               // 'r'
    Rotate,             // 'R'

    SetInputRadix,      // 'i'
    SetOutputRadix,     // 'o'
    SetPrecision,       // 'k'
    LoadInputRadix,     // 'I'
    LoadOutputRadix,    // 'O'
    LoadPrecision,      // 'K'

    Asciify,            // 'a'
    ExecuteMacro,       // 'x'

    Input,              // '?'
    Quit,               // 'q'
    QuitLevels,         // 'Q'

    NumDigits,          // 'Z'
    NumFrxDigits,       // 'X'
    StackDepth,         // 'z'

    ShellExec(String),  // '!'

    Version,            // '@'

    Eof,

    // Errors:
    Unimplemented(char),
    RegisterOutOfRange(char),
}

#[derive(Debug)]
pub enum RegisterAction {
    Store,              // 's'
    Load,               // 'l'
    PushRegStack,       // 'S'
    PopRegStack,        // 'L'
    Gt,                 // '>'
    Le,                 // '!>'
    Lt,                 // '<'
    Ge,                 // '!<'
    Eq,                 // '='
    Ne,                 // '!='
    StoreRegArray,      // ':'
    LoadRegArray,       // ';'
}

#[derive(Debug)]
enum ParseState {
    Any,
    Comment,
    Number { buf: String, decimal: bool },
    String { buf: String, level: usize },
    ShellExec(String),
    Bang,
    TwoChar(RegisterAction),

    /// 
    Unused(char),
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            state: Some(ParseState::Any),
        }
    }

    pub fn next(&mut self, mut input: impl Iterator<Item=char>) -> Action {
        let mut c = input.next();
        loop {
            if let Some(action) = self.step(&mut c) {
                return action;
            }
            if c.is_none() {
                c = input.next();
            }
        }
    }

    pub fn step(&mut self, c: &mut Option<char>) -> Option<Action> {
        debug!("current state: {:?}", self.state);
        let c = match self.state {
            Some(ParseState::Unused(c)) => {
                debug!("reusing unused input: {:?}", c);
                self.state = Some(ParseState::Any);
                c
            }
            _ => match c.take() {
                Some(c) => {
                    debug!("input: {:?}", c);
                    c
                }
                None => {
                    debug!("EOF");
                    return Some(Action::Eof);
                }
            }
        };
        let (new_state, result) = self.state.take().unwrap().next(c);
        debug!("new state: {:?}", new_state);
        debug!("result: {:?}", result);
        self.state = Some(new_state);
        result
    }
}

impl ParseState {
    pub fn next(self, c: char) -> (Self, Option<Action>) {
        match self {
            ParseState::Any => match c {
                ' ' | '\t' | '\r' | '\n' =>
                    (ParseState::Any, None),
                '_' | '0' ... '9' | 'A' ... 'F' | '.' =>
                    (ParseState::Number { buf: c.to_string(), decimal: c == '.' }, None),

                's' => (ParseState::TwoChar(RegisterAction::Store), None),
                'l' => (ParseState::TwoChar(RegisterAction::Load), None),
                'S' => (ParseState::TwoChar(RegisterAction::PushRegStack), None),
                'L' => (ParseState::TwoChar(RegisterAction::PopRegStack), None),
                '>' => (ParseState::TwoChar(RegisterAction::Gt), None),
                '<' => (ParseState::TwoChar(RegisterAction::Lt), None),
                '=' => (ParseState::TwoChar(RegisterAction::Eq), None),

                '!' => (ParseState::Bang, None),
                '[' => (ParseState::String { buf: String::new(), level: 0 }, None),
                '#' => (ParseState::Comment, None),

                'p' => (self, Some(Action::Print)),
                'n' => (self, Some(Action::PrintNoNewlinePop)),
                'P' => (self, Some(Action::PrintBytesPop)),
                'f' => (self, Some(Action::PrintStack)),

                '+' => (self, Some(Action::Add)),
                '-' => (self, Some(Action::Sub)),
                '*' => (self, Some(Action::Mul)),
                '/' => (self, Some(Action::Div)),
                '%' => (self, Some(Action::Rem)),
                '~' => (self, Some(Action::DivRem)),
                '^' => (self, Some(Action::Exp)),
                '|' => (self, Some(Action::ModExp)),
                'v' => (self, Some(Action::Sqrt)),

                'c' => (self, Some(Action::ClearStack)),
                'd' => (self, Some(Action::Dup)),
                'r' => (self, Some(Action::Swap)),
                'R' => (self, Some(Action::Rotate)),
                
                'i' => (self, Some(Action::SetInputRadix)),
                'o' => (self, Some(Action::SetOutputRadix)),
                'k' => (self, Some(Action::SetPrecision)),
                'I' => (self, Some(Action::LoadInputRadix)),
                'O' => (self, Some(Action::LoadOutputRadix)),
                'K' => (self, Some(Action::LoadPrecision)),

                'a' => (self, Some(Action::Asciify)),
                'x' => (self, Some(Action::ExecuteMacro)),

                '?' => (self, Some(Action::Input)),
                'q' => (self, Some(Action::Quit)),
                'Q' => (self, Some(Action::QuitLevels)),
                
                '@' => (self, Some(Action::Version)),

                _ => (self, Some(Action::Unimplemented(c))),
            },
            ParseState::Comment => match c {
                '\n' => (ParseState::Any, None),
                _ => (self, None),
            }
            ParseState::Number { mut buf, decimal } => match c {
                '_' =>
                    (ParseState::Number { buf: c.to_string(), decimal: false },
                        Some(Action::PushNumber(buf))),
                '0' ... '9' | 'A' ... 'F' => {
                    buf.push(c);
                    (ParseState::Number { buf, decimal }, None)
                }
                '.' if decimal =>
                    (ParseState::Number { buf: c.to_string(), decimal: true },
                        Some(Action::PushNumber(buf))),
                '.' if !decimal => {
                    buf.push(c);
                    (ParseState::Number { buf, decimal: true }, None)
                }
                _ => (ParseState::Unused(c), Some(Action::PushNumber(buf)))
            }
            ParseState::String { mut buf, level } => match c {
                '[' => {
                    buf.push(c);
                    (ParseState::String { buf, level: level + 1 }, None)
                }
                ']' if level > 0 => {
                    buf.push(c);
                    (ParseState::String { buf, level: level - 1 }, None)
                }
                ']' if level == 0 => (ParseState::Any, Some(Action::PushString(buf))),
                _ => {
                    buf.push(c);
                    (ParseState::String { buf, level }, None)
                }
            }
            ParseState::ShellExec(mut buf) => match c {
                '\n' => (ParseState::Any, Some(Action::ShellExec(buf))),
                _ => {
                    buf.push(c);
                    (ParseState::ShellExec(buf), None)
                }
            }
            ParseState::Bang => match c {
                '>' => (ParseState::TwoChar(RegisterAction::Le), None),
                '<' => (ParseState::TwoChar(RegisterAction::Ge), None),
                '=' => (ParseState::TwoChar(RegisterAction::Ne), None),

                _ => (ParseState::ShellExec(String::new()), None),
            }
            ParseState::TwoChar(action) => match c as u32 {
                0 ... 255 => (ParseState::Any, Some(Action::Register(action, c as u8))),
                _ => (ParseState::Any, Some(Action::RegisterOutOfRange(c)))
            }
            ParseState::Unused(_) => panic!("cannot next() on ParseState::Unused"),
        }
    }
}
