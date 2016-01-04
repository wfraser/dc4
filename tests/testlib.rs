//
// dc4 test suite
//
// Copyright (c) 2015-2016 by William R. Fraser
//

extern crate dc4;

use std::io::Cursor;

fn dc4_run(expr: &str) -> String {
    let mut dc = dc4::DC4::new("dc4 cargo test".to_string());
    let mut out = Vec::<u8>::new();

    dc.program(&mut Cursor::new(expr.as_bytes()), &mut out);

    String::from_utf8(out).unwrap()
}

#[test]
fn test_noop() {
    assert_eq!(dc4_run(""), "");
}

#[test]
fn test_at() {
    assert_eq!(dc4_run("@"), "dc4\n");
}

#[test]
fn test_f() {
    assert_eq!(dc4_run("1 2 3 f"), "3\n2\n1\n");
}

#[test]
fn test_input_radix() {
    assert_eq!(dc4_run("16i FFFF f"), "65535\n");
}

#[test]
fn test_output_radix() {
    assert_eq!(dc4_run("16o 65535 f"), "FFFF\n");
}

#[test]
fn test_weird_overflow() {
    // yes, this is actually what Unix dc does.
    // it doesn't check that digits are within the current input radix
    assert_eq!(dc4_run("12A3 f"), "1303\n");
}

#[test]
fn test_p() {
    assert_eq!(dc4_run("1 2 3 p"), "3\n");
    assert_eq!(dc4_run("1 2 [hello] p"), "hello\n");
    assert_eq!(dc4_run("p"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_n() {
    assert_eq!(dc4_run("1 2 3 n"), "3");
    assert_eq!(dc4_run("1 2 [hello] n"), "hello");
    assert_eq!(dc4_run("n"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_string_basic() {
    assert_eq!(dc4_run("[Hello, World!]f"), "Hello, World!\n");
}

#[test]
fn test_string_nesting() {
    assert_eq!(dc4_run("[Hello[World]]f"), "Hello[World]\n");
    assert_eq!(dc4_run("[[Hello]World]f"), "[Hello]World\n");
}

#[test]
fn test_negative() {
    assert_eq!(dc4_run("12_34_56 78 f"), "78\n-56\n-34\n12\n");
    assert_eq!(dc4_run("___f"), "0\n0\n0\n");
}

#[test]
fn test_invalid_radix() {
    {
        let error = "dc4 cargo test: input base must be a number between 2 and 16 (inclusive)\n";
        assert_eq!(dc4_run("1i f"), error);
        assert_eq!(dc4_run("17i f"), error);
        assert_eq!(dc4_run("_10i f"), error);
        assert_eq!(dc4_run("[bad]i f"), error);
    }
    {
        let error = "dc4 cargo test: output base must be a number greater than 1\n";
        assert_eq!(dc4_run("1o f"), error);
        assert_eq!(dc4_run("_10o f"), error);
        assert_eq!(dc4_run("[bad]o f"), error);
    }
}

#[test]
fn test_arithmetic() {
    assert_eq!(dc4_run("999 1 +f"), "1000\n");
    assert_eq!(dc4_run("1 2 3 ++f"), "6\n");
    assert_eq!(dc4_run("999 1 -f"), "998\n");
    assert_eq!(dc4_run("10 20 -f"), "-10\n");
    assert_eq!(dc4_run("_15 32 +f"), "17\n");
    assert_eq!(dc4_run("5 3 *f"), "15\n");
    assert_eq!(dc4_run("50 5 /f"), "10\n");
    assert_eq!(dc4_run("51 5 /f"), "10\n");
    assert_eq!(dc4_run("_51 5 /f"), "-10\n");
    assert_eq!(dc4_run("51 _5 /f"), "-10\n");
    assert_eq!(dc4_run("5 50 /f"), "0\n");
    assert_eq!(dc4_run("53 5 %f"), "3\n");
    assert_eq!(dc4_run("53 5 ~f"), "3\n10\n");
    assert_eq!(dc4_run("2 10 ^f"), "1024\n");
    assert_eq!(dc4_run("_2 10 ^f"), "1024\n");
    assert_eq!(dc4_run("2 0 ^f"), "1\n");
    assert_eq!(dc4_run("2 _10 ^f"), "0\n");
}

#[test]
fn test_invalid_arithmetic() {
    assert_eq!(dc4_run("[shoe] 7 *f"), "dc4 cargo test: non-numeric value\n7\nshoe\n");
    assert_eq!(dc4_run("7[shoe] *f"),  "dc4 cargo test: non-numeric value\nshoe\n7\n");
    assert_eq!(dc4_run("3 0 /f"), "dc4 cargo test: divide by zero\n0\n3\n");
    assert_eq!(dc4_run("3 0 %f"), "dc4 cargo test: divide by zero\n0\n3\n");
    assert_eq!(dc4_run("3 0 ~f"), "dc4 cargo test: divide by zero\n0\n3\n");
}

#[test]
fn test_registers() {
    assert_eq!(dc4_run("42 99 sx f lx f"), "42\n99\n42\n");
    assert_eq!(dc4_run("lxf"), "dc4 cargo test: register 'x' (0170) is empty\n");
    assert_eq!(dc4_run("sxf"), "dc4 cargo test: stack empty\n");
    assert_eq!(dc4_run("42 ss f"), ""); // checks for a bug in handling 2-char commands
}

#[test]
fn test_register_stack() {
    assert_eq!(dc4_run("1 2 3 f SxSx f LxLx f"), "3\n2\n1\n1\n3\n2\n1\n");
    assert_eq!(dc4_run("Lxf"), "dc4 cargo test: stack register 'x' (0170) is empty\n");
    assert_eq!(dc4_run("Sxf"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_stackmanip() {
    assert_eq!(dc4_run("1 2 3 frf"), "3\n2\n1\n2\n3\n1\n");
    assert_eq!(dc4_run("1 2 3 fdf"), "3\n2\n1\n3\n3\n2\n1\n");
    assert_eq!(dc4_run("1 2 3 f c 4 f"), "3\n2\n1\n4\n");
}

#[test]
fn test_macro() {
    assert_eq!(dc4_run("4 5 [d+p] x f"), "10\n10\n4\n");
    assert_eq!(dc4_run("25 x f"), "25\n");
    //assert_eq!(dc4_run("[ok]ss[lsp]st9_9<t"), "ok\n");
}

#[test]
fn test_conditional_macro() {
    assert_eq!(dc4_run("1 1 [[hello]n]sx =x f"), "hello");
    assert_eq!(dc4_run("1 2 [[hello]n]sx =x f"), "");
    assert_eq!(dc4_run("1 2 [[hello]n]sx !=x f"), "hello");

    assert_eq!(dc4_run("1 2 [[hello]n]sx >x"), "hello");
    assert_eq!(dc4_run("2 1 [[hello]n]sx >x"), "");
    assert_eq!(dc4_run("2 1 [[hello]n]sx !>x"), "hello");

    assert_eq!(dc4_run("2 1 [[hello]n]sx <x"), "hello");
    assert_eq!(dc4_run("1 2 [[hello]n]sx <x"), "");
    assert_eq!(dc4_run("1 2 [[hello]n]sx !<x"), "hello");

    assert_eq!(dc4_run("1 1 =x 2 f"), "dc4 cargo test: register 'x' (0170) is empty\n2\n");

    assert_eq!(dc4_run("1 1 2 3 [[hello]n]sx !=x=x"), "hellohello");
    assert_eq!(dc4_run("1 2 [[hello]n]sx ! =x"), "");
}

#[test]
fn test_array() {
    assert_eq!(dc4_run("7 [hello] 42:x f c 42;x f"), "7\nhello\n");
    assert_eq!(dc4_run("7 [hello] [bogus] :x f"), "dc4 cargo test: array index must be a nonnegative integer\n7\n");
    assert_eq!(dc4_run("42 ;x f"), "0\n");
    assert_eq!(dc4_run(";x f"), "dc4 cargo test: stack empty\n");
    assert_eq!(dc4_run("[bogus];x f"), "dc4 cargo test: array index must be a nonnegative integer\n");

    assert_eq!(dc4_run("1 0:a 0Sa 2 0:a La 0;a f"), "1\n0\n");
}

#[test]
fn test_print_ascii() {
    let program =
        // "Test passed." in ASCII.
        "84 101 115 116 32 112 97 115 115 101 100 46".to_string()
        + "zsn"                 // save stack size to 'n'
        + "[z:xz0<y]dsyx"       // put the stack into array 'x'
        + "1[d;xP1+dln!<z]dszx" // print array 'x' as ASCII characters
        + "10P";                // print a newline

    assert_eq!(dc4_run(&program), "Test passed.\n");
}

#[test]
fn test_quitlevels() {
    let program = String::new()
        + "5"                   // 5 times through the loop
        + "[2Q]sq"              // macro to quit 2 levels
        + "["
            + "d3=q"            // on 3, call the quit macro
            + "1-ddn0<x"        // subtract 1, print it, and if >0, loop again
        + "]dsxx"
        + "[done]p";

    // virtual stack frames when the q macro is called:
    // 3
    // 4
    // main

    // This is a neat test because with tail recursion, 3 and 4 are actually in the same stack
    // frame, and without precautions, the 2Q will quit the main frame as well.

    assert_eq!(dc4_run(&program), "43done\n");
}

#[test]
#[ignore] // because this test is so slow. be sure to run 'cargo test -- --ignored' occasionally.
fn test_stackoverflow() {
    let iterations = "200000";

    let program = String::new()
        + "[pq]sq"      // 'q' macro to print and quit
        + "0"           // start counter
        + "["
            + "1+"                    // increment the counter
            + "d" + iterations + "=q" // if the counter hits the magic number, invoke the 'q' macro
            + "lmx"                   // invoke ourselves
        + "]dsmx";                    // store to 'm' and execute

    assert_eq!(dc4_run(&program), iterations.to_string() + "\n");
}

#[test]
fn test_frac_output() {
    assert_eq!(dc4_run("2k 50 3 /f"), "16.66\n");
    assert_eq!(dc4_run("5k 16o 3 10 /f"), ".4CCCC\n");
    assert_eq!(dc4_run("2k 2o 1 2 /f"), ".1000000\n");
}
