//
// dc4 input parser
//
// Copyright (c) 2019 by William R. Fraser
//

pub struct Parser {
    state: Option<ParseState>,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            state: Some(ParseState::Start),
        }
    }
}

#[derive(Debug)]
pub enum Action {
    // Where possible, keep things ordered like in the GNU dc man page.

    // Numbers and strings have eac been split into two operations, to avoid having any buffering
    // in the parser. The expectation is that these Actions will not be interleaved with any others.
    // Also it can be assumed that any sequence of number character actions will always be valid.
    NumberChar(char),
    StringChar(char),
    PushNumber,
    PushString,

    Register(RegisterAction, char),

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

    /// NOTE: DC4 purposely does not implement this or buffer the command to be executed.
    ShellExec,          // '!'

    /// DC4 extension.
    Version,            // '@'

    /// End of input was reached.
    Eof,

    // Errors:

    /// Unimplemented (or unrecognized) command.
    Unimplemented(char),

    /// Something went wrong reading or parsing input.
    InputError(String),
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
    Start,
    Comment,
    Number { decimal: bool },
    String { level: usize },
    ShellExec,
    Bang,
    TwoChar(RegisterAction),
}

impl Parser {
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

    pub fn step(&mut self, input: &mut Option<char>) -> Option<Action> {
        let (new_state, result) = self.state.take().unwrap().next(input);
        self.state = Some(new_state);
        result
    }
}

impl ParseState {
    /// Given the current state and an input character, return the new state and maybe an Action.
    /// If `input` is None after this call, it means the character was consumed. If not, it should
    /// be re-issued again.
    pub fn next(self, input: &mut Option<char>) -> (Self, Option<Action>) {
        let c = match input.take() {
            Some(c) => c,
            None => {
                // We are at EOF. We need to complete whatever we're in the middle of, or return
                // Action::Eof to positively indicate that we're done.
                let action: Action = match self {
                    ParseState::Start => Action::Eof,
                    ParseState::Comment => Action::Eof,
                    ParseState::Number { decimal: _ } => Action::PushNumber,
                    ParseState::String { level: _ } =>
                        // Note: we push the string even if it is incomplete (unbalanced brackets).
                        Action::PushString,
                    ParseState::ShellExec => Action::ShellExec,
                    ParseState::Bang =>
                        // GNU dc interprets this as an empty shell command and tries to run it
                        // This is pointless, so let's just ignore it.
                        Action::Eof,
                    ParseState::TwoChar(_register_action) =>
                        Action::InputError("unexpected end of input".into())
                };
                return (ParseState::Start, Some(action))
            }
        };

        match self {
            ParseState::Start => match c {
                // Where possible, keep things ordered like in the GNU dc man page.

                ' ' | '\t' | '\r' | '\n' =>
                    (self, None),

                '_' | '0' ... '9' | 'A' ... 'F' | '.' =>
                    (ParseState::Number { decimal: c == '.' }, Some(Action::NumberChar(c))),

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

                's' => (ParseState::TwoChar(RegisterAction::Store), None),
                'l' => (ParseState::TwoChar(RegisterAction::Load), None),
                'S' => (ParseState::TwoChar(RegisterAction::PushRegStack), None),
                'L' => (ParseState::TwoChar(RegisterAction::PopRegStack), None),

                'i' => (self, Some(Action::SetInputRadix)),
                'o' => (self, Some(Action::SetOutputRadix)),
                'k' => (self, Some(Action::SetPrecision)),
                'I' => (self, Some(Action::LoadInputRadix)),
                'O' => (self, Some(Action::LoadOutputRadix)),
                'K' => (self, Some(Action::LoadPrecision)),

                '[' => (ParseState::String { level: 0 }, None),
                'a' => (self, Some(Action::Asciify)),
                'x' => (self, Some(Action::ExecuteMacro)),

                '!' => (ParseState::Bang, None),
                '>' => (ParseState::TwoChar(RegisterAction::Gt), None),
                '<' => (ParseState::TwoChar(RegisterAction::Lt), None),
                '=' => (ParseState::TwoChar(RegisterAction::Eq), None),
                '?' => (self, Some(Action::Input)),
                'q' => (self, Some(Action::Quit)),
                'Q' => (self, Some(Action::QuitLevels)),

                'Z' => (self, Some(Action::NumDigits)),
                'X' => (self, Some(Action::NumFrxDigits)),
                'z' => (self, Some(Action::StackDepth)),

                '#' => (ParseState::Comment, None),
                ':' => (ParseState::TwoChar(RegisterAction::StoreRegArray), None),
                ';' => (ParseState::TwoChar(RegisterAction::LoadRegArray), None),

                '@' => (self, Some(Action::Version)),

                _ => (self, Some(Action::Unimplemented(c))),
            },
            ParseState::Comment => match c {
                '\n' => (ParseState::Start, None),
                _ => (self, None),
            }
            ParseState::Number { decimal } => match c {
                '0' ... '9' | 'A' ... 'F' => {
                    (ParseState::Number { decimal }, Some(Action::NumberChar(c)))
                }
                '.' if !decimal => {
                    (ParseState::Number { decimal: true }, Some(Action::NumberChar(c)))
                }
                _ => {
                    // Any of: a negative sign while we're already in a number, or a decimal sign
                    // when we've already seen one, or any other non-number character. These all end
                    // the current number and return us to the start state, but the character must
                    // be handled on the next iteration. Put the character back in the input Option
                    // to signal that we want it again next time.
                    *input = Some(c);
                    (ParseState::Start, Some(Action::PushNumber))
                }
            }
            ParseState::String { level } => match c {
                '[' => {
                    (ParseState::String { level: level + 1 }, Some(Action::StringChar(c)))
                }
                ']' if level > 0 => {
                    (ParseState::String { level: level - 1 }, Some(Action::StringChar(c)))
                }
                ']' if level == 0 => (ParseState::Start, Some(Action::PushString)),
                _ => {
                    (ParseState::String { level }, Some(Action::StringChar(c)))
                }
            }
            ParseState::ShellExec => match c {
                '\n' => (ParseState::Start, Some(Action::ShellExec)),
                _ => {
                    (ParseState::ShellExec, None)
                }
            }
            ParseState::Bang => match c {
                '>' => (ParseState::TwoChar(RegisterAction::Le), None),
                '<' => (ParseState::TwoChar(RegisterAction::Ge), None),
                '=' => (ParseState::TwoChar(RegisterAction::Ne), None),
                _ => (ParseState::ShellExec, None),
            }
            ParseState::TwoChar(action) => (ParseState::Start, Some(Action::Register(action, c))),
        }
    }
}
