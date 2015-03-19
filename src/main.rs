#![feature(core)]
#![feature(env)]
#![feature(collections)]

extern crate dc4;
extern crate collections;

use std::env;
use collections::borrow::ToOwned;

/*
fn basename(path: &str) -> &str {

    let mut i = path.len();
    for c in path.chars().rev() {
        if c == '/' {
            return &path[i..];
        }

        i -= 1;
    }

    path
}
*/

fn basename(path: &str) -> &str {
    match path.split('/').rev().next() {
        Some(s) => s,
        _ => path
    }
}

fn progname() -> String {
    basename(env::args().next().expect("no program name?!").as_slice()).to_owned()
}

fn print_version() {
    println!("{}: version 1", progname());
}

fn print_usage() {
    println!("usage: {} [options and stuff]", progname());
}

fn main() {
    let expression_str = "--expression=";
    let file_str = "--file=";

    let mut process_stdin = true;
    let args: Vec<String> = env::args().collect();
    let mut skip = 0;
    
    for i in 0..args.len() {

        if skip > 0 {
            skip -= 1;
            continue;
        }

        let arg = args[i].as_slice();

        if arg == "-V" || arg == "--version" {
           print_version();
           return;
        }
        else if arg == "-h" || arg == "--help" {
            print_usage();
            return;
        }
        else if arg == "-e" {
            if i + 1 == args.len() {
                println!("\"-e\" must be followed by an argument.");
                return;
            }

            let p = args[i + 1].as_slice();
            println!("process expression: {}", p);
            dc4::program(p);
            skip = 1;
            process_stdin = false;
        }
        else if arg.len() > expression_str.len()
                && &arg[..expression_str.len()] == expression_str.as_slice() {
            let p = &arg[expression_str.len()..];

            println!("process expression: {}", p);
            dc4::program(p);
            process_stdin = false;
        }
        else if arg == "-f" {
            if i + 1 == args.len() {
                println!("\"-f\" must be followed by an argument.");
                return;
            }

            let p = args[i + 1].as_slice();
            println!("process file: {}", p);
            skip = 1;
            process_stdin = false;
        }
        else if arg.len() > file_str.len()
                && &arg[..file_str.len()] == file_str.as_slice() {
            let p = &arg[file_str.len()..];

            println!("process file: {}", p);
            process_stdin = false;
        }
        else if i != 0 {
            //TODO read file
            println!("process file {}", arg);
            process_stdin = false;
        }
    }


    if process_stdin {
        println!("process stdin");
    }
}
