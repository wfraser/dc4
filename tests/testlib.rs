extern crate dc4;

use std::io::Cursor;

fn dc4_run(expr: &str) -> String {
    let mut dc = dc4::DC4::new();
    let mut out = Vec::<u8>::new();

    dc.program(&mut Cursor::new(expr.as_bytes()), &mut out);

    String::from_utf8(out).unwrap()
}

#[test]
fn test_noop() {
    assert_eq!(dc4_run(""), "");
}

#[test]
fn test_f() {
    assert_eq!(dc4_run("1 2 3 f"), "3\n2\n1\n");
}
