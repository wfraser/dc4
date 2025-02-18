//
// dc4 test suite
//
// Copyright (c) 2015-2021 by William R. Fraser
//

#![deny(rust_2018_idioms)]

use dc4::Flavor::{self, *};

fn dc4_run(expr: &[u8]) -> String {
    dc4_run_v(Gnu, expr)
}

fn dc4_run_v(flavor: Flavor, expr: &[u8]) -> String {
    String::from_utf8(dc4_run_bytes_v(flavor, expr)).unwrap()
}

fn dc4_run_bytes(expr: &[u8]) -> Vec<u8> {
    dc4_run_bytes_v(Gnu, expr)
}

fn dc4_run_bytes_v(flavor: Flavor, expr: &[u8]) -> Vec<u8> {
    let mut dc = dc4::Dc4::new("dc4 cargo test".to_string(), flavor);
    let mut out = Vec::<u8>::new();

    dc.text(expr.to_vec(), &mut out);

    out
}

fn dc4_run_two(expr1: &[u8], expr2: &[u8]) -> String {
    let mut dc = dc4::Dc4::new("dc4 cargo test".to_string(), Gnu);
    let mut out = Vec::<u8>::new();

    dc.text(expr1.to_vec(), &mut out);
    dc.text(expr2.to_vec(), &mut out);

    String::from_utf8(out).unwrap()
}

#[test]
fn test_noop() {
    assert_eq!(dc4_run(b""), "");
}

#[test]
fn test_at() {
    let ver = env!("CARGO_PKG_VERSION_MAJOR").parse::<u64>().unwrap() << 24
            | env!("CARGO_PKG_VERSION_MINOR").parse::<u64>().unwrap() << 16
            | env!("CARGO_PKG_VERSION_PATCH").parse::<u64>().unwrap();
    assert_eq!(dc4_run(b"@f"), format!("dc4\n{ver}\n"));
    assert_eq!(dc4_run(b"@r0+"), ""); // ensure the version is a number
}

#[test]
fn test_f() {
    assert_eq!(dc4_run(b"1 2 3 f"), "3\n2\n1\n");
}

#[test]
fn test_input_radix() {
    assert_eq!(dc4_run(b"16i FFFF f"), "65535\n");
}

#[test]
fn test_output_radix() {
    assert_eq!(dc4_run(b"16o 65535 f"), "FFFF\n");
}

#[test]
fn test_weird_overflow() {
    // yes, this is actually what Unix dc does.
    // it doesn't check that digits are within the current input radix
    assert_eq!(dc4_run(b"12A3 f"), "1303\n");
}

#[test]
fn test_p() {
    assert_eq!(dc4_run(b"1 2 3 p"), "3\n");
    assert_eq!(dc4_run(b"1 2 [hello] p"), "hello\n");
    assert_eq!(dc4_run(b"p"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_n() {
    assert_eq!(dc4_run(b"1 2 3 n"), "3");
    assert_eq!(dc4_run(b"1 2 [hello] n"), "hello");
    assert_eq!(dc4_run(b"n"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_string_basic() {
    assert_eq!(dc4_run(b"[Hello, World!]f"), "Hello, World!\n");
}

#[test]
fn test_string_nesting() {
    assert_eq!(dc4_run(b"[Hello[World]]f"), "Hello[World]\n");
    assert_eq!(dc4_run(b"[[Hello]World]f"), "[Hello]World\n");
}

#[test]
fn test_negative() {
    assert_eq!(dc4_run(b"12_34_56 78 f"), "78\n-56\n-34\n12\n");
    assert_eq!(dc4_run(b"___f"), "0\n0\n0\n");
}

#[test]
fn test_invalid_radix() {
    {
        let error = "dc4 cargo test: input base must be a number between 2 and 16 (inclusive)\n";
        assert_eq!(dc4_run(b"1i f"), error);
        assert_eq!(dc4_run(b"17i f"), error);
        assert_eq!(dc4_run(b"_10i f"), error);
        assert_eq!(dc4_run(b"[bad]i f"), error);
    }
    {
        let error = "dc4 cargo test: output base must be a number between 2 and 16 (inclusive)\n";
        assert_eq!(dc4_run(b"1o f"), error);
        assert_eq!(dc4_run(b"_10o f"), error);
        assert_eq!(dc4_run(b"[bad]o f"), error);
    }
}

#[test]
fn test_arithmetic() {
    assert_eq!(dc4_run(b"999 1 +f"), "1000\n");
    assert_eq!(dc4_run(b"1 2 3 ++f"), "6\n");
    assert_eq!(dc4_run(b"999 1 -f"), "998\n");
    assert_eq!(dc4_run(b"10 20 -f"), "-10\n");
    assert_eq!(dc4_run(b"_15 32 +f"), "17\n");
    assert_eq!(dc4_run(b"5 3 *f"), "15\n");
    assert_eq!(dc4_run(b"50 5 /f"), "10\n");
    assert_eq!(dc4_run(b"51 5 /f"), "10\n");
    assert_eq!(dc4_run(b"_51 5 /f"), "-10\n");
    assert_eq!(dc4_run(b"51 _5 /f"), "-10\n");
    assert_eq!(dc4_run(b"5 50 /f"), "0\n");
    assert_eq!(dc4_run(b"53 5 %f"), "3\n");
    assert_eq!(dc4_run(b"53 5 ~f"), "3\n10\n");
    assert_eq!(dc4_run(b"2 10 ^f"), "1024\n");
    assert_eq!(dc4_run(b"_2 10 ^f"), "1024\n");
    assert_eq!(dc4_run(b"2 0 ^f"), "1\n");
    assert_eq!(dc4_run(b"2 _10 ^f"), "0\n");
    assert_eq!(dc4_run(b"12k 2 _10 ^f"), ".000976562500\n");
    assert_eq!(dc4_run(b"10k _2 _9 ^f"), "-.0019531250\n");
}

#[test]
fn test_invalid_arithmetic() {
    assert_eq!(dc4_run(b"[shoe] 7 *f"), "dc4 cargo test: non-numeric value\n7\nshoe\n");
    assert_eq!(dc4_run(b"7[shoe] *f"),  "dc4 cargo test: non-numeric value\nshoe\n7\n");
    assert_eq!(dc4_run(b"3 0 /f"), "dc4 cargo test: divide by zero\n0\n3\n");
    assert_eq!(dc4_run(b"3 0 %f"), "dc4 cargo test: remainder by zero\n0\n3\n");
    assert_eq!(dc4_run(b"3 0 ~f"), "dc4 cargo test: divide by zero\n0\n3\n");
    assert_eq!(dc4_run(b"3 2.5 ^f"), "dc4 cargo test: warning: non-zero scale in exponent\n9\n");
}

#[test]
fn test_registers() {
    assert_eq!(dc4_run(b"42 99 sx f lx f"), "42\n99\n42\n");
    assert_eq!(dc4_run(b"lxf"), "dc4 cargo test: register 'x' (0170) is empty\n");
    assert_eq!(dc4_run(b"sxf"), "dc4 cargo test: stack empty\n");
    assert_eq!(dc4_run(b"42 ss f"), ""); // checks for a bug in handling 2-char commands
}

#[test]
fn test_register_stack() {
    assert_eq!(dc4_run(b"1 2 3 f SxSx f LxLx f"), "3\n2\n1\n1\n3\n2\n1\n");
    assert_eq!(dc4_run(b"Lxf"), "dc4 cargo test: stack register 'x' (0170) is empty\n");
    assert_eq!(dc4_run(b"Sxf"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_stackmanip() {
    assert_eq!(dc4_run(b"1 2 3 frf"), "3\n2\n1\n2\n3\n1\n");
    assert_eq!(dc4_run(b"1 2 3 fdf"), "3\n2\n1\n3\n3\n2\n1\n");
    assert_eq!(dc4_run(b"1 2 3 f c 4 f"), "3\n2\n1\n4\n");
}

#[test]
fn test_macro() {
    assert_eq!(dc4_run(b"4 5 [d+p] x f"), "10\n10\n4\n");
    assert_eq!(dc4_run(b"25 x f"), "25\n");
    //assert_eq!(dc4_run("[ok]ss[lsp]st9_9<t"), "ok\n");
}

#[test]
fn test_conditional_macro() {
    assert_eq!(dc4_run(b"1 1 [[hello]n]sx =x f"), "hello");
    assert_eq!(dc4_run(b"1 2 [[hello]n]sx =x f"), "");
    assert_eq!(dc4_run(b"1 2 [[hello]n]sx !=x f"), "hello");

    assert_eq!(dc4_run(b"1 2 [[hello]n]sx >x"), "hello");
    assert_eq!(dc4_run(b"2 1 [[hello]n]sx >x"), "");
    assert_eq!(dc4_run(b"2 1 [[hello]n]sx !>x"), "hello");

    assert_eq!(dc4_run(b"2 1 [[hello]n]sx <x"), "hello");
    assert_eq!(dc4_run(b"1 2 [[hello]n]sx <x"), "");
    assert_eq!(dc4_run(b"1 2 [[hello]n]sx !<x"), "hello");

    assert_eq!(dc4_run(b"1 1 =x 2 f"), "dc4 cargo test: register 'x' (0170) is empty\n2\n");

    assert_eq!(dc4_run(b"1 1 2 3 [[hello]n]sx !=x=x"), "hellohello");
    assert_eq!(dc4_run(b"1 2 [[hello]n]sx !=x"), "hello");
}

#[test]
fn test_array() {
    assert_eq!(dc4_run(b"7 [hello] 42:x f c 42;x f"), "7\nhello\n");
    assert_eq!(dc4_run(b"7 [hello] [bogus] :x f"), "dc4 cargo test: array index must be a nonnegative integer\n7\n");
    assert_eq!(dc4_run(b"42 ;x f"), "0\n");
    assert_eq!(dc4_run(b";x f"), "dc4 cargo test: stack empty\n");
    assert_eq!(dc4_run(b"[bogus];x f"), "dc4 cargo test: array index must be a nonnegative integer\n");

    assert_eq!(dc4_run(b"1 0:a 0Sa 2 0:a La 0;a f"), "1\n0\n");
}

#[test]
fn test_print_ascii() {
    let program = concat!(
        // "Test passed." in ASCII.
        "84 101 115 116 32 112 97 115 115 101 100 46",
        "zsn",                  // save stack size to 'n'
        "[z:xz0<y]dsyx",        // put the stack into array 'x'
        "1[d;xP1+dln!<z]dszx",  // print array 'x' as ASCII characters
        "10P",                  // print a newline
    );

    assert_eq!(dc4_run(program.as_bytes()), "Test passed.\n");
}

#[test]
fn test_quitlevels() {
    let program = concat!(
        "5",                    // 5 times through the loop
        "[2Q]sq",               // macro to quit 2 levels
        "[",
            "d3=q",             // on 3, call the quit macro
            "1-ddn0<x",         // subtract 1, print it, and if >0, loop again
        "]dsxx",
        "[done]p",
    );

    // virtual stack frames when the q macro is called:
    // 3
    // 4
    // main

    // This is a neat test because with tail recursion, 3 and 4 are actually in the same stack
    // frame, and without precautions, the 2Q will quit the main frame as well.

    assert_eq!(dc4_run(program.as_bytes()), "43done\n");
}

#[test]
fn test_quitlevels2() {
    let program = concat!(
        "19 20 21 22",          // some values to accumulate
        "[2Q]sq",               // macro to quit 2 levels
        "[",
            "z1=q",             // call quit macro when the stack depth is 1 (no more to accumulate)
            "+",                // otherwise, add the top two numbers
            "0_=x",             // unconditionally execute this macro again
        "]dsxx",
        "f",                    // write the stack at the end
    );

    // The [2Q] will be executed when the 'x' macro has run 3 times.
    // Even though it says to quit 2 levels, and we're at a virtual stack depth of 3, it needs to
    // quit out of 'x' entirely, because it's *tail* recursion: there's nothing to be done once a
    // level exits.

    assert_eq!(dc4_run(program.as_bytes()), "82\n");
}

#[test]
fn test_quitlevels3() {
    assert_eq!(dc4_run(b"[[[[q]x1p]x2p]x3p]x4p"), "2\n3\n4\n");
    assert_eq!(dc4_run(b"[q]s1 [l1x]s2 [l2x]s3 l3x [three]p l2x [two]p l1x [one]p"), "three\ntwo\n");
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

    assert_eq!(dc4_run(program.as_bytes()), iterations.to_string() + "\n");
}

#[test]
fn test_frac_output() {
    assert_eq!(dc4_run(b"2k 50 3 /f"), "16.66\n");
    assert_eq!(dc4_run(b"5k 16o 3 10 /f"), ".4CCCC\n");
    assert_eq!(dc4_run(b"2k 2o 1 2 /f"), ".1000000\n");
}

#[test]
fn test_small_print() {
    assert_eq!(dc4_run(b"5k 50 3 %f"), ".00002\n");
}

#[test]
fn test_decimal() {
    assert_eq!(dc4_run(b"12.345 f"), "12.345\n");
    assert_eq!(dc4_run(b"12. f"), "12\n");
    assert_eq!(dc4_run(b"12.34.56 f"), ".56\n12.34\n");
    assert_eq!(dc4_run(b".1234f"), ".1234\n");
    assert_eq!(dc4_run(b".f"), "0\n");
    assert_eq!(dc4_run(b"..f"), "0\n0\n");

    // A dc number's precision is the number of digits it has, which is then interpreted as
    // specifying *decimal* digits, no matter what the input radix is. So you get weird stuff like:
    assert_eq!(dc4_run(b"16i 1.F f"), "1.9\n");
    assert_eq!(dc4_run(b"16i 1.F0 f"), "1.93\n");
    assert_eq!(dc4_run(b"16i 1.F00 f"), "1.937\n");
    assert_eq!(dc4_run(b"16i 1.F000 f"), "1.9375\n");
    assert_eq!(dc4_run(b"16i 1.F0000 f"), "1.93750\n");

    // test math with mixed precisions
    assert_eq!(dc4_run(b"10.5 7 *f"), "73.5\n");
    assert_eq!(dc4_run(b"1.2 1.002 +f"), "2.202\n");
}

#[test]
fn test_utf8() {
    assert_eq!(dc4_run("[Ā‡🎅]f sa f la f".as_bytes()), "Ā‡🎅\nĀ‡🎅\n");
    assert_eq!(dc4_run("[[Ā‡🎅]f]x".as_bytes()), "Ā‡🎅\n");
    assert_eq!(dc4_run("[🎅]s🎅".as_bytes()),
        "dc4 cargo test: \'\\u{9f}\' (0237) unimplemented\n\
        dc4 cargo test: \'\\u{8e}\' (0216) unimplemented\n\
        dc4 cargo test: \'\\u{85}\' (0205) unimplemented\n");

    // now some invalid UTF8 in input, which is allowed:
    assert!(dc4_run_bytes(b"42 [\xc3\x28] f") == b"\xc3\x28\n42\n");
    assert!(dc4_run_bytes(b"[\xf8\xa1\xa1\xa1\xa1]f") == b"\xf8\xa1\xa1\xa1\xa1\n");
}

#[test]
fn test_modexp() {
    assert_eq!(dc4_run(b"4 13 497 |f"), "445\n");
    assert_eq!(dc4_run(b"4 _13 497 |f"), "dc4 cargo test: negative exponent\n497\n-13\n4\n");
    assert_eq!(dc4_run(b"4 13.9 497 |f"), "dc4 cargo test: warning: non-zero scale in exponent\n445\n");
    assert_eq!(dc4_run(b"4 13 0 |f"), "dc4 cargo test: remainder by zero\n0\n13\n4\n");
    assert_eq!(dc4_run(b"16o 16i 2946288212CAA2D5B80E1C661006807F 3285C3432ACBCB0F4D0232282ECC73DB 267D2F2E51C216A7DA752EAD48D22D89 |f"),
        "DDC404D916005967425A8D8A066CA56\n");
}

#[test]
fn test_sqrt() {
    assert_eq!(dc4_run(b"[foo] vf"), "dc4 cargo test: square root of nonnumeric attempted\n");
    assert_eq!(dc4_run(b"_25 vf"), "dc4 cargo test: square root of negative number\n");
    assert_eq!(dc4_run(b"0 vf"), "0\n");

    assert_eq!(dc4_run(b"25 vf"), "5\n");
    assert_eq!(dc4_run(b"25.000 vf"), "5.000\n");
    assert_eq!(dc4_run(b"3k 25 vf"), "5.000\n");
    assert_eq!(dc4_run(b"5k 25.000 vf"), "5.00000\n");
    assert_eq!(dc4_run(b"3k 25.00000 vf"), "5.00000\n");
    assert_eq!(dc4_run(b"15241.384 vf"), "123.456\n");
    assert_eq!(dc4_run(b"15241.383 vf"), "123.455\n");

    assert_eq!(dc4_run(b"16o 15241.384 vf"), "7B.74B\n");            // 123.455
    assert_eq!(dc4_run(b"16o 15241.383 vf"), "7B.747\n");            // 123.454
    assert_eq!(dc4_run(b"2o 15241.384 vf"), "1111011.0111010010\n"); // 123.4550781250
    assert_eq!(dc4_run(b"2o 15241.383 vf"), "1111011.0111010001\n"); // 123.4541015625
}

#[test]
fn test_comment() {
    assert_eq!(dc4_run(b"1 2 # 3 4 \n 5 6 f"), "6\n5\n2\n1\n");
    assert_eq!(dc4_run(b"1 2 [# 3 4] 5 6 f"), "6\n5\n# 3 4\n2\n1\n");
    assert_eq!(dc4_run(b"1 2 # [3\n4] 5\n6 f"), "dc4 cargo test: \']\' (0135) unimplemented\n6\n5\n4\n2\n1\n");
}

#[test]
fn test_odd_registers() {
    assert_eq!(dc4_run(b"[[foo]p]s# 0 0=#"), "foo\n"); // use the register named '#', not comment
    assert_eq!(dc4_run(b"[[foo]p]s\n 0 0=\n"), "foo\n"); // whitespace counts for once
    assert_eq!(dc4_run(b"[[foo]p]s 0 0= "), "foo\n"); // ditto
    assert_eq!(dc4_run(b"[[foo]p]s! 0 0=!"), "foo\n"); // don't trigger shell command parsing
    assert_eq!(dc4_run(b"[[foo]p]s< 0 0=<"), "foo\n");
}

#[test]
fn test_shell() {
    // this tests a couple things:
    //   1. ! followed by space followed by an equality check should NOT get interpreted as a
    //      negative equality check, it should be recognized as a shell execute command.
    //   2. the rest of the line should be ignored
    //   3. that the shell command is not run, obviously
    assert_eq!(dc4_run(b"1 2 [[oops]n]sx ! =x [oops2]p\n[hello]p"), "dc4 cargo test: running shell commands is not supported\nhello\n");
}

#[test]
fn test_char_print_with_scale() {
    assert_eq!(dc4_run(b"3k 37 P"), "%");
}

#[test]
fn test_char_print_order() {
    assert_eq!(dc4_run(b"4276803P"), "ABC");
    assert_eq!(dc4_run(b"4276803.99P"), "ABC");
    assert_eq!(dc4_run(b"_4276803.99P"), "ABC");
    assert_eq!(dc4_run(b"16i 303132 P"), "012");
}

#[test]
fn test_a() {
    assert_eq!(dc4_run(b"4276803af"), "C\n");
    assert_eq!(dc4_run(b"[hello]af"), "h\n");
    assert_eq!(dc4_run(b"[]af"), "\n");
    assert_eq!(dc4_run(b"a"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_huge_input_dec() {
    let s = "123456787901234567890123456789012345678901234567890123456789012345678901234567890".to_owned();
    assert_eq!(dc4_run((s.clone() + "f").as_bytes()), s + "\n");
}

#[test]
fn test_huge_input_hex() {
    let s = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".to_owned();
    assert_eq!(dc4_run(("16o 16i ".to_owned() + &s + "f").as_bytes()), s + "\n");
}

#[test]
fn test_frx_digit_count() {
    assert_eq!(dc4_run(b".000450Xf"), "6\n");
    assert_eq!(dc4_run(b"123.000450Xf"), "6\n");
    assert_eq!(dc4_run(b"123.000450 10000000* Xf"), "6\n");
    assert_eq!(dc4_run(b"[spaghetti]Xf"), "0\n");
    assert_eq!(dc4_run(b"Xf"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_digit_count() {
    assert_eq!(dc4_run(b".000450Zf"), "3\n");
    assert_eq!(dc4_run(b"123.000450Zf"), "9\n");
    assert_eq!(dc4_run(b"123.000450 10000000* Zf"), "16\n");
    assert_eq!(dc4_run(b"[spoopadoop]Zf"), "10\n");
    assert_eq!(dc4_run(b"Zf"), "dc4 cargo test: stack empty\n");
}

#[test]
fn test_parser_tricky() {
    // This checks for an edge case in the parser where it can lose the last character in input
    // because it is both EOF and also has a left-over character from the 'f' in "16f" resulting in
    // an action and also a stashed character.
    assert_eq!(dc4_run(b"16ff"), "16\n16\n");

    // This checks that partial strings at the end of input are pushed anyway.
    assert_eq!(dc4_run_two(b"[partial", b"f"), "partial\n");

    // This checks that in-progress numbers are pushed at the end of input.
    assert_eq!(dc4_run_two(b"1234", b"f"), "1234\n");

    // This checks that an incomplete two-character action at the end of input triggers an error.
    assert_eq!(dc4_run_two(b"1234s", b"f"), "dc4 cargo test: error reading input: unexpected end of file\n1234\n");

    // This checks that comments don't somehow spill over into subsequent inputs.
    assert_eq!(dc4_run_two(b"1234#", b"5678f"), "5678\n1234\n");
}

#[test]
fn test_zero_print() {
    // prints "0", not ".000" like you'd think
    assert_eq!(dc4_run(b"12.345 .345- 12- f"), "0\n");

    // but the scale didn't actually change:
    assert_eq!(dc4_run(b"12.345 .345- 12- .1+ f"), ".100\n");
}

#[test]
fn test_obase_neg_frac() {
    assert_eq!(dc4_run(b"_1.5 16of"), "-1.8\n");
}

#[test]
fn test_large_obase() {
    assert_eq!(dc4_run(b"1 100of"), " 01\n");
    assert_eq!(dc4_run(b"1 101of"), " 001\n");
    assert_eq!(dc4_run(b"1024 64of"), " 16 00\n");
    assert_eq!(dc4_run(b"5120 64of"), " 01 16 00\n");
    assert_eq!(dc4_run(b"123456 101of"), " 012 010 034\n");
    assert_eq!(dc4_run(b"123456.789 101of"), " 012 010 034.079 069\n");
    assert_eq!(dc4_run(b"_123456.789 16of"), "-1E240.C9F\n");
    assert_eq!(dc4_run(b"_123456.789 101of"), "- 012 010 034.079 069\n");
    assert_eq!(dc4_run(b"0.789 101of"), ".079 069\n");
}

#[test]
fn test_string_escaped_brackets() {
    assert_eq!(dc4_run(b"[foo]f"), "foo\n");
    assert_eq!(dc4_run(b"[\\]foo]f"), "]foo\n");
    assert_eq!(dc4_run(b"[\\[foo]f"), "[foo\n");
    assert_eq!(dc4_run(b"[\\\\foo]f"), "\\foo\n");
}

#[test]
fn test_ifelse() {
    assert_eq!(dc4_run_v(Bsd, b"[[r1]p]sx [[r2]p]sy 1 _1 =xey"), "r2\n");
    assert_eq!(dc4_run_v(Bsd, b"[[r1]p]sx [[r2]p]sy _1 _1 =xey"), "r1\n");
    assert_eq!(dc4_run_v(Bsd, b"[[r1]p]sx [[r2]p]sy 1 _1 =xe"), "dc4 cargo test: error reading input: unexpected end of file\n");
    assert_eq!(dc4_run(b"[[r1]p]sx [[r2]p]sy _1 _1 =x"), "r1\n");

    assert_eq!(dc4_run_v(Bsd, b"[[r1]p]sx [[r2]p]sy 1 _1 !=xey"), "r1\n");
    assert_eq!(dc4_run_v(Bsd, b"[[r1]p]sx [[r2]p]sy _1 _1 !=xey"), "r2\n");
    assert_eq!(dc4_run_v(Bsd, b"[[r1]p]sx [[r2]p]sy 1 _1 !=xe"), "dc4 cargo test: error reading input: unexpected end of file\n");
    assert_eq!(dc4_run(b"[[r1]p]sx [[r2]p]sy 1 _1 !=x"), "r1\n");
}

#[test]
fn test_compares() {
    assert_eq!(dc4_run_v(Bsd, b"7 _7 Gf"), "0\n");
    assert_eq!(dc4_run_v(Bsd, b"7 7 Gf"), "1\n");
    assert_eq!(dc4_run_v(Bsd, b"[foo] 7 Gf"), "dc4 cargo test: non-numeric value\n7\nfoo\n");

    assert_eq!(dc4_run_v(Bsd, b"7 Nf"), "0\n");
    assert_eq!(dc4_run_v(Bsd, b"0 Nf"), "1\n");
    assert_eq!(dc4_run_v(Bsd, b"0.000 Nf"), "1\n");
    assert_eq!(dc4_run_v(Bsd, b"[foo] 7 Gf"), "dc4 cargo test: non-numeric value\n7\nfoo\n");

    assert_eq!(dc4_run_v(Bsd, b"7 _7 (f"), "1\n");
    assert_eq!(dc4_run_v(Bsd, b"7 7 (f"), "0\n");
    assert_eq!(dc4_run_v(Bsd, b"[foo] 7 (f"), "dc4 cargo test: non-numeric value\n7\nfoo\n");

    assert_eq!(dc4_run_v(Bsd, b"7 _7 {f"), "1\n");
    assert_eq!(dc4_run_v(Bsd, b"7 7 {f"), "1\n");
    assert_eq!(dc4_run_v(Bsd, b"[foo] 7 {f"), "dc4 cargo test: non-numeric value\n7\nfoo\n");

    assert_eq!(dc4_run_v(Gavin, b"_7 7 )f"), "1\n");
    assert_eq!(dc4_run_v(Gavin, b"7 7 )f"), "0\n");
    assert_eq!(dc4_run_v(Gavin, b"[foo] 7 )f"), "dc4 cargo test: non-numeric value\n7\nfoo\n");

    assert_eq!(dc4_run_v(Gavin, b"_7 7 }f"), "1\n");
    assert_eq!(dc4_run_v(Gavin, b"7 7 }f"), "1\n");
    assert_eq!(dc4_run_v(Gavin, b"[foo] 7 }f"), "dc4 cargo test: non-numeric value\n7\nfoo\n");
}
