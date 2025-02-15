use dc4::parser::Action;
use dc4::reader_parser::ReaderParser;
use std::io::{self, Cursor, Read};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let input: Box<dyn Read> = if args.is_empty() {
        Box::new(io::stdin())
    } else if args == "--help" || args == "-h" {
        eprintln!(
            "usage: {} [expression...]",
            std::env::args().next().unwrap()
        );
        eprintln!("prints the dc parse of [expression], or the expression read from stdin if the");
        eprintln!("    command-line is empty.");
        std::process::exit(1);
    } else {
        Box::new(Cursor::new(args))
    };
    print_parse(input, 0);
}

fn print_parse(input: impl Read, indent: usize) {
    let pre = " ".repeat(indent * 4);
    let parser = ReaderParser::new(input);
    let mut pending = vec![];
    for action in parser {
        match action {
            Action::NumberChar(c) | Action::StringChar(c) => pending.push(c),
            Action::PushNumber | Action::PushString => {
                let s = String::from_utf8_lossy(&pending);
                println!("{pre}{action:?}({s:?})");
                if matches!(action, Action::PushString) {
                    print_parse(Cursor::new(&pending), indent + 1);
                }
                pending = vec![];
            }
            Action::Register(a, r) => println!("{pre}Register({a:?}, {:?})", char::from(r)),
            Action::Unimplemented(c) => println!("{pre}Unimplemented({:?})", char::from(c)),
            _ => println!("{pre}{action:?}"),
        }
    }
}
