//
// dc4 :: A Unix dc(1) implementation in Rust.
//
// This is the program entry point.
// It parses command line arguments and invokes the dc4 library.
//
// Copyright (c) 2015 by William R. Fraser
//

extern crate dc4;

use std::env;

fn basename(path: &str) -> &str {
    match path.split('/').rev().next() {
        Some(s) => s,
        _ => path
    }
}

fn progname() -> String {
    basename(env::args().next().expect("no program name?!").as_ref()).to_owned()
}

fn print_version() {
    println!("{}: version 1", progname());
}

fn print_usage() {
    println!("usage: {} [options and stuff]", progname());
}

enum DCInput<'a> {
    Expression(&'a str),
    File(&'a str),
    Stdin,
}

fn parse_arguments<'a>(args: &'a Vec<String>) -> Option<Vec<DCInput<'a>>> {
    let mut inputs: Vec<DCInput<'a>> = Vec::new();

    let expression_str = "--expression=";
    let file_str = "--file=";

    let mut process_stdin = true;
    let mut seen_double_dash = false;

    let mut skip = 0; // number of args to skip next time around
    for i in 0..args.len() {

        if skip > 0 {
            skip -= 1;
            continue;
        }

        let arg = &args[i];

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
            inputs.push(DCInput::File(p));
            skip = 1;
            process_stdin = false;
        }
        else if arg == "--" {
            seen_double_dash = true;
        }
        else if arg == "-" {
            inputs.push(DCInput::Stdin);
            process_stdin = false;
        }
        else if arg.len() > file_str.len()
                && &arg[..file_str.len()] == file_str {

            let p = &arg[file_str.len()..];
            inputs.push(DCInput::File(p));
            process_stdin = false;
        }
        else if i != 0 {
            inputs.push(DCInput::File(arg));
            process_stdin = false;
        }
    }

    if process_stdin {
        inputs.push(DCInput::Stdin);
    }


    Some(inputs)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut inputs: Vec<DCInput>;
    
    match parse_arguments(&args) {
        Some(x) => inputs = x,
        None => return,
    }

    let mut dc = dc4::DC4::new();

    for input in inputs {
        match input {
            DCInput::Expression(expr) => {
                println!("process expression {}", expr);
                dc.program(expr);
            },
            DCInput::File(file) => {
                //TODO read file
                println!("process file {}", file);
            },
            DCInput::Stdin => {
                //TODO read stdin
                println!("process stdin");
            },
        }
    }
}
