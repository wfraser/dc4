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
}

#[test]
fn test_n() {
    assert_eq!(dc4_run("1 2 3 n"), "3");
}

#[test]
fn test_string_basic() {
    assert_eq!(dc4_run("[Hello, World!]f"), "Hello, World!\n");
}

#[test]
fn test_string_nesting() {
    assert_eq!(dc4_run("[Hello[World]]f"), "Hello[World]\n");
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
