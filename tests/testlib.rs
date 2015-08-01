extern crate dc4;

#[test]
fn noop() {
    assert_eq!(true, true);
}

#[test]
#[allow(unused_variables)]
fn test_instantiate() {
    let dc = dc4::DC4::new();
}
