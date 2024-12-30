//
// dc4 :: A Unix dc(1) implementation in Rust.
//
// This is the program entry point.
// It parses command line arguments and invokes the dc4 library.
//
// Copyright (c) 2015-2024 by William R. Fraser
//

#![deny(rust_2018_idioms)]

use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

use dc4::Dc4;
use dc4::DcResult;

fn progname() -> String {
    Path::new(env::args_os().next().expect("no program name?!").as_os_str())
        .file_stem().expect("no program name?!")
        .to_string_lossy()
        .into_owned()
}

fn print_version() {
    println!("dc4 version {}", env!("CARGO_PKG_VERSION"));
    println!("Copyright (c) 2015-2024 by William R. Fraser");
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
enum DcInput<'a> {
    Expression(&'a str),
    File(&'a str),
    Stdin,
}

fn parse_arguments<'a>(args: &'a [&'a str])
        -> Option<Vec<DcInput<'a>>> {
    let mut inputs: Vec<DcInput<'a>> = Vec::new();
    let mut bare_file_args: Vec<DcInput<'a>> = Vec::new();

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
            inputs.push(DcInput::File(arg));
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
            inputs.push(DcInput::Expression(p));

            skip = 1;
            process_stdin = false;
        }
        else if arg.len() > expression_str.len()
                && &arg[..expression_str.len()] == expression_str {
            let p = &arg[expression_str.len()..];

            inputs.push(DcInput::Expression(p));
            process_stdin = false;
        }
        else if arg == "-f" {
            if i + 1 == args.len() {
                println!("\"-f\" must be followed by an argument.");
                return None;
            }

            let p = &args[i + 1];
            if !seen_double_dash && p == &"-" {
                inputs.push(DcInput::Stdin);
            } else {
                inputs.push(DcInput::File(p));
            }
            skip = 1;
            process_stdin = false;
        }
        else if arg == "--" {
            seen_double_dash = true;
        }
        else if arg == "-" {
            bare_file_args.push(DcInput::Stdin);
            process_stdin = false;
        }
        else if arg.len() > file_str.len()
                && &arg[..file_str.len()] == file_str {

            let p = &arg[file_str.len()..];
            inputs.push(DcInput::File(p));
            process_stdin = false;
        }
        else if i != 0 {
            bare_file_args.push(DcInput::File(arg));
            process_stdin = false;
        }
    }

    inputs.append(&mut bare_file_args);

    if process_stdin {
        inputs.push(DcInput::Stdin);
    }

    Some(inputs)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_references: Vec<&str> = args.iter().map(|owned| &owned[..]).collect();

    let inputs: Vec<DcInput<'_>> = match parse_arguments(&args_references) {
        Some(x) => x,
        None => return,
    };

    let mut dc = Dc4::new(progname());

    for input in inputs {
        let result = match input {
            DcInput::Expression(expr) => {
                dc.text(expr.as_bytes().to_vec(), &mut io::stdout())
            },
            DcInput::File(path) => {
                match File::open(path) {
                    Ok(file) => dc.stream(&mut std::io::BufReader::new(file), &mut io::stdout()),
                    Err(e)       => {
                        println!("{}: File open failed on {:?}: {}", progname(), path, e);
                        DcResult::Terminate(0)
                    }
                }
            },
            DcInput::Stdin => {
                let stdin = io::stdin();
                let mut lock = stdin.lock();
                dc.stream(&mut lock, &mut io::stdout())
            },
        };

        match result {
            DcResult::Macro(_) => panic!("unhandled macro"),
            DcResult::Terminate(_) => return,
            DcResult::QuitLevels(_) // if there are quit levels left at the end of an input, they
                                    // are ignored.
                | DcResult::Continue
                => (),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parseargs() {
        let args: Vec<&str> = vec!["-e", "e1", "file1", "--expression=e2", "file2", "--file=file3", "-", "file4"];
        let result = parse_arguments(&args).unwrap();

        // first, the options:
        assert_eq!(result[0], DcInput::Expression("e1"));
        assert_eq!(result[1], DcInput::Expression("e2"));
        assert_eq!(result[2], DcInput::File("file3"));

        // then the non-option inputs:
        assert_eq!(result[3], DcInput::File("file1"));
        assert_eq!(result[4], DcInput::File("file2"));
        assert_eq!(result[5], DcInput::Stdin);
        assert_eq!(result[6], DcInput::File("file4"));

        assert_eq!(result.len(), 7);
    }
}
