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
