use proc_macro::*;
use std::path::Path;

#[derive(serde::Deserialize)]
struct CargoManifest {
    package: CargoPackage,
}

#[derive(serde::Deserialize)]
struct CargoPackage {
    edition: String,
}

struct DeleteOnDrop<P: AsRef<Path>> {
    path: P,
}
impl<P: AsRef<Path>> DeleteOnDrop<P> {
    fn new(path: P) -> Self {
        DeleteOnDrop {
            path
        }
    }
}
impl<P: AsRef<Path>> Drop for DeleteOnDrop<P> {
    fn drop(&mut self) {
        _ = std::fs::remove_file(self.path.as_ref());
    }
}

#[proc_macro]
/// This runs arbitrary code at compile time.
/// To do this, it creates a file of rust,
/// compiles it to a binary, runs the binary,
/// then deletes both.
/// # Usage
/// There are two arguments to this macro:
/// 1. The file name
/// 2. The code
/// 
/// ### File Name
/// The file name is the name of the file created
/// (specifically there will be (name).rs and (name)
/// because there is the rust file and the binary)
/// Because this creates file with that name, if there
/// are pre-existing file with that name, they will
/// be overwritten. If you don't want that to happen,
/// I suggest giving them a unique name.
/// (important side note: because this is a macro,
/// it takes in the code you give it as is, meaning
/// that if you give do method("file_name", code),
/// it will fail because idk, it doesn't like having
/// a file called "file_name".rs)
///```
/// # use abes_nice_procs::method;
/// # fn main() {
/// method!(file_name, fn main() {})
/// // Creates file_name.rs and file_name
/// //          ^^^ provided code  ^^^ binary
/// # }
///```
/// ### Code
/// The code is placed into the file directly as given,
/// meaning it needs a main function to be placed in it.
/// The best way to think of it(because this is what happens)
/// is as if you are making a main file, meaning that it
/// needs all the things associated with that.
/// This also means that you can create functions inside it.
///```
/// # use abes_nice_procs::method;
/// # fn main() {
/// method!(example,
///     // Start of section put into file
///     fn main() {
///         helper();
///     }
///     fn helper() {
///         // Code here
///     }
///     // End of section put into file
/// )
/// # } 
///```
/// This wouldn't be a macro if it didn't create code.
/// The way this actually determines gets the code is weird.
/// It uses the stdout of the code given to it.
/// Meaning that instead of printing to the terminal,
/// you are essentially printing to your file.
///```
/// # use abes_nice_procs::method;
/// # fn main() {
/// assert_eq!(method!(example2,
///     fn main() {
///         print!("5");
///         //     ^^^ Notice how the 5 is in quotes
///     }
/// ), 5);
/// # }
///```
/// Even though the 5 of the macro was in quotes
/// and is therefore a [String],
/// it was still able to be compared to the integer 5.
/// This is because the output of the macro is given as code.
/// Meaning that the compiler saw the integer literal 5
/// and acted accordingly.
/// 
/// Similarly, you could
///```
/// # use abes_nice_procs::method;
/// # fn main() {
/// assert_eq!(method!(example3,
///     fn main() {
///         print!("\"Hello\"");
///         //     ^^^ Notice the escaped quotes
///     }
/// ), "Hello")
/// # }
///```
/// Just like before when the quotes got removed,
/// when trying to put a [String] in,
/// the quotes will still be removed.
/// But that can be bypassed by escaping out the quotes.
pub fn method(attr: TokenStream) -> TokenStream {
    // Getting path
    let mut trees = attr.into_iter();
    let path = if let TokenTree::Ident(ident) = trees.next().unwrap() {
        ident.to_string()
    } else {
        panic!("could not get path")
    };

    // Checking format
    if !matches!(trees.next(), Some(TokenTree::Punct(p)) if p.as_char() == ',') {
        panic!("expected comma after filename");
    }

    // Getting code
    let code = trees.collect::<TokenStream>().to_string();

    // Getting edition
    let cargo_toml_content = std::fs::read_to_string("Cargo.toml").expect("failed to load Cargo.toml");
    let manifest = toml::from_str::<CargoManifest>(&cargo_toml_content).expect("failed to parse Cargo.toml");
    let edition = &manifest.package.edition;

    let rs_path = format!("{path}.rs");
    std::fs::write(&rs_path, code).expect("failed to make file");
    let _rs_path_guard = DeleteOnDrop::new(&rs_path);

    let compile_status = std::process::Command::new("rustc")
        .arg(&rs_path)
        .arg("--edition")
        .arg(edition)
        .spawn()// Allows getting input from the terminal
        .and_then(|mut c| c.wait())
        .expect("failed to compile");
    if !compile_status.success() {
        panic!("failed to compile: {compile_status}")
    }

    let bin_path = Path::new(".").join(&path);// Have to do this for windows compatability
    let _bin_path_guard = DeleteOnDrop::new(&bin_path);
    // This section has to be before we run the command becasue running it could fail
    // and the bin would be left undeleted

    let output = std::process::Command::new(&bin_path).output().expect("failed to run file");
    if !output.status.success() {
        panic!("failed to run file: {}", output.status);
    }

    String::from_utf8(output.stdout)
        .unwrap()
        .parse::<TokenStream>()
        .unwrap()
}