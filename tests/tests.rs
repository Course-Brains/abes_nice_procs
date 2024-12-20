use abes_nice_procs::*;

#[test]
fn test() {
    assert_eq!(method!(here,
        fn main() {
            print!("5");
        }
    ), 5);
}
#[derive(Test)]
pub struct Asd<T> {
    help: usize,
    banan: Option<Vec<i128>>,
    nawr: T
}