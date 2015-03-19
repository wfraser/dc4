//#![feature(old_io)]
#![feature(os)]
#![feature(core)]

extern crate dc4;

//use std::old_io;
use std::os;

fn print_version() {
    println!("{}: version 1", os::args()[0]);
}

fn print_usage() {
    println!("{}: usage", os::args()[0]);
}

fn main() {
    let mut process_stdin = true;
    let args = os::args();
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
