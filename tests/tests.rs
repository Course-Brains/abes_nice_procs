use abes_nice_procs::*;

#[test]
fn test() {
    method!(here,
        fn main() {
            std::fs::write("output.md", "yooooooooooooo!");
        }
    )
}