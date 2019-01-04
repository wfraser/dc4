//
// dc4 :: A Unix dc(1) implementation in Rust.
//
// This is the program entry point.
// It parses command line arguments and invokes the dc4 library.
//
// Copyright (c) 2015-2018 by William R. Fraser
//

extern crate dc4;
extern crate env_logger;

use std::env;
use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;

use dc4::DC4;
use dc4::DCResult;

fn progname() -> String {
    Path::new(env::args_os().next().expect("no program name?!").as_os_str())
        .file_stem().expect("no program name?!")
        .to_string_lossy()
        .into_owned()
}

fn print_version() {
    println!("dc4 version {}", env!("CARGO_PKG_VERSION"));
    println!("Copyright (c) 2015-2018 by William R. Fraser");
}

fn print_usage() {
    // 79:    ###############################################################################
    println!("usage: {} [options] [file ...]", progname());
    println!("options:");
    println!("  -e EXPR | --expression=EXPR     evaluate expression");
    println!("  -f FILE | --file=FILE           evaluate contents of file");
    println!("  -h | --help                     display this help and exit");
    println!("  -V | --version                  output version information and exit");
    println!();
    println!("Expressions from command line options are processed first, in order, followed");
    println!("by any remaining files listed. A file name of '-' means to read from standard");
    println!("input. An argument of '--' disables further command line option processing and");
    println!("all subsequent arguments are interpreted as file names. If no inputs are given,");
    println!("input will be taken from standard input.");
}

#[derive(Debug, PartialEq)]
enum DCInput<'a> {
    Expression(&'a str),
    File(&'a str),
    Stdin,
}

fn parse_arguments<'a>(args: &'a [&'a str])
        -> Option<Vec<DCInput<'a>>> {
    let mut inputs: Vec<DCInput<'a>> = Vec::new();
    let mut bare_file_args: Vec<DCInput<'a>> = Vec::new();

    let expression_str = "--expression=";
    let file_str = "--file=";

    let mut process_stdin = true;
    let mut seen_double_dash = false;

    let mut skip = 0; // number of args to skip next time around
    for (i, arg) in args.iter().cloned().enumerate() {

        if skip > 0 {
            skip -= 1;
            continue;
        }

        if seen_double_dash {
            inputs.push(DCInput::File(arg));
            process_stdin = false;
        }
        else if arg == "-V" || arg == "--version" {
           print_version();
           return None;
        }
        else if arg == "-h" || arg == "--help" {
            print_usage();
            return None;
        }
        else if arg == "-e" {
            if i + 1 == args.len() {
                println!("\"-e\" must be followed by an argument.");
                return None;
            }

            let p = &args[i + 1];
            inputs.push(DCInput::Expression(p));

            skip = 1;
            process_stdin = false;
        }
        else if arg.len() > expression_str.len()
                && &arg[..expression_str.len()] == expression_str {
            let p = &arg[expression_str.len()..];

            inputs.push(DCInput::Expression(p));
            process_stdin = false;
        }
        else if arg == "-f" {
            if i + 1 == args.len() {
                println!("\"-f\" must be followed by an argument.");
                return None;
            }

            let p = &args[i + 1];
            if !seen_double_dash && p == &"-" {
                inputs.push(DCInput::Stdin);
            } else {
                inputs.push(DCInput::File(p));
            }
            skip = 1;
            process_stdin = false;
        }
        else if arg == "--" {
            seen_double_dash = true;
        }
        else if arg == "-" {
            bare_file_args.push(DCInput::Stdin);
            process_stdin = false;
        }
        else if arg.len() > file_str.len()
                && &arg[..file_str.len()] == file_str {

            let p = &arg[file_str.len()..];
            inputs.push(DCInput::File(p));
            process_stdin = false;
        }
        else if i != 0 {
            bare_file_args.push(DCInput::File(arg));
            process_stdin = false;
        }
    }

    inputs.append(&mut bare_file_args);

    if process_stdin {
        inputs.push(DCInput::Stdin);
    }

    Some(inputs)
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let args_references: Vec<&str> = args.iter().map(|owned| &owned[..]).collect();

    let inputs: Vec<DCInput> = match parse_arguments(&args_references) {
        Some(x) => x,
        None => return,
    };

    let mut dc = DC4::new(progname());

    for input in inputs {
        let result = match input {
            DCInput::Expression(expr) => {
                dc.program(&mut Cursor::new(expr.as_bytes()), &mut io::stdout())
            },
            DCInput::File(path) => {
                match File::open(path) {
                    Ok(file) => dc.program(&mut std::io::BufReader::new(file), &mut io::stdout()),
                    Err(e)       => {
                        println!("{}: File open failed on {:?}: {}", progname(), path, e);
                        DCResult::Terminate(0)
                    }
                }
            },
            DCInput::Stdin => {
                let stdin = io::stdin();
                let mut lock = stdin.lock();
                dc.program(&mut lock, &mut io::stdout())
            },
        };

        match result {
            DCResult::Macro(_) => panic!("unhandled macro"),
            DCResult::Terminate(_) => return,
            DCResult::QuitLevels(_) => (),  // nothing: if there are quit levels left at the end of
                                            // an input, they are ignored.
            DCResult::Continue => ()
        }
    }
}

#[test]
fn test_parseargs() {
    let args: Vec<&str> = vec!["-e", "e1", "file1", "--expression=e2", "file2", "--file=file3", "-", "file4"];
    let result = parse_arguments(&args).unwrap();

    // first, the options:
    assert_eq!(result[0], DCInput::Expression("e1"));
    assert_eq!(result[1], DCInput::Expression("e2"));
    assert_eq!(result[2], DCInput::File("file3"));

    // then the non-option inputs:
    assert_eq!(result[3], DCInput::File("file1"));
    assert_eq!(result[4], DCInput::File("file2"));
    assert_eq!(result[5], DCInput::Stdin);
    assert_eq!(result[6], DCInput::File("file4"));

    assert_eq!(result.len(), 7);
}
