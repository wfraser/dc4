//
// dc4 input parser
//
// Copyright (c) 2019-2025 by William R. Fraser
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
    NumberChar(u8),
    StringChar(u8),
    PushNumber,
    PushString,

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
    Unimplemented(u8),

    /// Something went wrong reading or parsing input.
    InputError(std::io::Error),
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
    String { level: usize, bs: bool },
    ShellExec,
    Bang,
    TwoChar(RegisterAction),
}

impl Parser {
    pub fn step(&mut self, input: &mut Option<u8>) -> Option<Action> {
        let (new_state, result) = self.state.take().unwrap().next(input);
        self.state = Some(new_state);
        result
    }
}

impl ParseState {
    /// Given the current state and an input character, return the new state and maybe an Action.
    /// If `input` is None after this call, it means the character was consumed. If not, it should
    /// be re-issued again.
    pub fn next(self, input: &mut Option<u8>) -> (Self, Option<Action>) {
        let Some(c) = input.take() else {
            // We are at EOF. We need to complete whatever we're in the middle of, or return
            // Action::Eof to positively indicate that we're done.
            let action: Action = match self {
                ParseState::Start
                    | ParseState::Comment
                    | ParseState::Bang  // GNU dc interprets this as an empty shell command and
                                        // tries to execute it. This is pointless, so let's just
                                        // ignore it.
                    => Action::Eof,
                ParseState::Number { .. } => Action::PushNumber,
                ParseState::String { .. } =>
                    // Note: we push the string even if it is incomplete (unbalanced brackets).
                    Action::PushString,
                ParseState::ShellExec => Action::ShellExec,
                ParseState::TwoChar(_register_action) =>
                    Action::InputError(std::io::ErrorKind::UnexpectedEof.into())
            };
            return (ParseState::Start, Some(action));
        };

        match self {
            ParseState::Start => match c {
                // Where possible, keep things ordered like in the GNU dc man page.

                b' ' | b'\t' | b'\r' | b'\n' =>
                    (self, None),

                b'_' | b'0' ..= b'9' | b'A' ..= b'F' | b'.' =>
                    (ParseState::Number { decimal: c == b'.' }, Some(Action::NumberChar(c))),

                b'p' => (self, Some(Action::Print)),
                b'n' => (self, Some(Action::PrintNoNewlinePop)),
                b'P' => (self, Some(Action::PrintBytesPop)),
                b'f' => (self, Some(Action::PrintStack)),

                b'+' => (self, Some(Action::Add)),
                b'-' => (self, Some(Action::Sub)),
                b'*' => (self, Some(Action::Mul)),
                b'/' => (self, Some(Action::Div)),
                b'%' => (self, Some(Action::Rem)),
                b'~' => (self, Some(Action::DivRem)),
                b'^' => (self, Some(Action::Exp)),
                b'|' => (self, Some(Action::ModExp)),
                b'v' => (self, Some(Action::Sqrt)),

                b'c' => (self, Some(Action::ClearStack)),
                b'd' => (self, Some(Action::Dup)),
                b'r' => (self, Some(Action::Swap)),

                b's' => (ParseState::TwoChar(RegisterAction::Store), None),
                b'l' => (ParseState::TwoChar(RegisterAction::Load), None),
                b'S' => (ParseState::TwoChar(RegisterAction::PushRegStack), None),
                b'L' => (ParseState::TwoChar(RegisterAction::PopRegStack), None),

                b'i' => (self, Some(Action::SetInputRadix)),
                b'o' => (self, Some(Action::SetOutputRadix)),
                b'k' => (self, Some(Action::SetPrecision)),
                b'I' => (self, Some(Action::LoadInputRadix)),
                b'O' => (self, Some(Action::LoadOutputRadix)),
                b'K' => (self, Some(Action::LoadPrecision)),

                b'[' => (ParseState::String { level: 0, bs: false }, None),
                b'a' => (self, Some(Action::Asciify)),
                b'x' => (self, Some(Action::ExecuteMacro)),

                b'!' => (ParseState::Bang, None),
                b'>' => (ParseState::TwoChar(RegisterAction::Gt), None),
                b'<' => (ParseState::TwoChar(RegisterAction::Lt), None),
                b'=' => (ParseState::TwoChar(RegisterAction::Eq), None),
                b'?' => (self, Some(Action::Input)),
                b'q' => (self, Some(Action::Quit)),
                b'Q' => (self, Some(Action::QuitLevels)),

                b'Z' => (self, Some(Action::NumDigits)),
                b'X' => (self, Some(Action::NumFrxDigits)),
                b'z' => (self, Some(Action::StackDepth)),

                b'#' => (ParseState::Comment, None),
                b':' => (ParseState::TwoChar(RegisterAction::StoreRegArray), None),
                b';' => (ParseState::TwoChar(RegisterAction::LoadRegArray), None),

                b'@' => (self, Some(Action::Version)),

                _ => (self, Some(Action::Unimplemented(c))),
            },
            ParseState::Comment => match c {
                b'\n' => (ParseState::Start, None),
                _ => (self, None),
            }
            ParseState::Number { decimal } => match c {
                b'0' ..= b'9' | b'A' ..= b'F' => {
                    (ParseState::Number { decimal }, Some(Action::NumberChar(c)))
                }
                b'.' if !decimal => {
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
            ParseState::String { level, bs } => match c {
                b'\\' if !bs => (ParseState::String { level, bs: true }, None),
                b'[' if !bs => (
                    ParseState::String { level: level + 1, bs: false },
                    Some(Action::StringChar(c))
                ),
                b']' if !bs && level > 0 => (
                    ParseState::String { level: level - 1, bs: false },
                    Some(Action::StringChar(c))
                ),
                b']' if !bs && level == 0 => (ParseState::Start, Some(Action::PushString)),
                _ => (ParseState::String { level, bs: false }, Some(Action::StringChar(c))),
            }
            ParseState::ShellExec => match c {
                b'\n' => (ParseState::Start, Some(Action::ShellExec)),
                _ => (ParseState::ShellExec, None),
            }
            ParseState::Bang => match c {
                b'>' => (ParseState::TwoChar(RegisterAction::Le), None),
                b'<' => (ParseState::TwoChar(RegisterAction::Ge), None),
                b'=' => (ParseState::TwoChar(RegisterAction::Ne), None),
                _ => (ParseState::ShellExec, None),
            }
            ParseState::TwoChar(action) => (ParseState::Start, Some(Action::Register(action, c))),
        }
    }
}
