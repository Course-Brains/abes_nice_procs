use abes_nice_procs::*;

#[test]
fn test() {
    assert_eq!(method!(here,
        fn main() {
            print!("5");
        }
    ), 5);
}