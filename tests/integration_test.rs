// Cargo knows to look for integration test files in this directory
use dataprep as _;

#[test]
fn it_adds_two() {
    assert_eq!(4, dataprep::add_two(2));
}
