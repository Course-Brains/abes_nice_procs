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
enum What {
    Struct,
    Enum
}
impl What {
    fn from_ident(ident: Ident) -> Option<What> {
        match ident.to_string().as_str() {
            "struct" => Some(What::Struct),
            "enum" => Some(What::Enum),
            _ => None
        }
    }
}
impl std::fmt::Display for What {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            What::Struct => write!(f, "struct"),
            What::Enum => write!(f, "enum")
        }
    }
}
struct DeriveData {
    what: What,
    name: Ident,
    generic: Vec<TokenTree>,
    fields: Vec<Field>
}
impl DeriveData {
    fn implement(&self, which: Which) -> String {
        let mut out = String::new();
        match which {
            Which::From => {
                out += "impl";
                out += &self.generic.iter().map(|x| x.to_string()).collect::<String>();
                out += " FromBinary for ";
                out += &self.name.to_string();
                // Second generic definition
                for generic in self.generic.split(|x| {
                    if let TokenTree::Punct(punct) = x {
                        if punct.to_string() == "," {
                            return true;
                        }
                    }
                    return false;
                }) {
                    'inner: for token in generic.iter() {
                        if let TokenTree::Punct(punct) = token {
                            if punct.to_string() == ":" {
                                break 'inner
                            }
                        }
                        out += &token.to_string();
                    }
                    out += ",";
                }
                out.pop();
                out += "{ fn from_binary(binary: &mut dyn std::io::Read) -> Self {";
                for field in self.fields.iter() {
                    out += "self.";
                    out += &field.name;
                    out += "=";
                    out += &field.data_type;
                    out += "::from_binary(binary),"
                }
                out += "}}";
            }
            Which::To => {
                // ToBinary
                out += "impl";
                out += &self.generic.iter().map(|x| x.to_string()).collect::<String>();
                out += " ToBinary for ";
                out += &self.name.to_string();
                for generic in self.generic.split(|x| {
                    if let TokenTree::Punct(punct) = x {
                        if punct.to_string() == "," {
                            return true;
                        }
                    }
                    return false;
                }) {
                    'inner: for token in generic.iter() {
                        if let TokenTree::Punct(punct) = token {
                            if punct.to_string() == ":" {
                                break 'inner
                            }
                        }
                        out += &token.to_string();
                    }
                    out += ",";
                }
                out.pop();
                out += "{ fn to_binary(self, write: &mut dyn std::io::Write) {";
                for field in self.fields.iter() {
                    out += "self.";
                    out += &field.name;
                    out += ".to_binary(write);"
                }
                out += "}}";
            }
        }
        return out
    }
}
impl From<TokenStream> for DeriveData {
    fn from(value: TokenStream) -> Self {
        let mut iter = value.into_iter();
        let mut what: Option<What> = None;
        while let Some(token) = iter.next() {
            if let TokenTree::Ident(ident) = token {
                if let Some(wht) = What::from_ident(ident) {
                    what = Some(wht);
                    break;
                }
            }
        }
        let what = what.expect("Missing what it is(struct/enum)");
        let name_tree = iter.next().expect("Missing name");
        let name;
        if let TokenTree::Ident(ident) = name_tree {
            name = ident;
        }
        else {
            panic!("FUCK FUCK FUCK FUCK FUCK FUCK")
        }
        let mut generic = Vec::new();
        let mut fields_stream: Option<Vec<TokenTree>> = None;
        while let Some(token) = iter.next() {
            if let TokenTree::Group(group) = token {
                fields_stream = Some(group.stream().into_iter().collect());
                break;
            }
            else {
                generic.push(token);
            }
        }
        let fields_stream = fields_stream.expect("Could not get fields");
        let mut fields = Vec::new();
        for field_tokens in fields_stream.split(|x| {
            if let TokenTree::Punct(punct) = x {
                if punct.to_string() == ",".to_string() {
                    return true
                }
            }
            return false
        }) {
            fields.push(Field {
                name: field_tokens[0].to_string(),
                data_type: {
                    field_tokens[2..].iter().map(|x| x.to_string()).collect::<String>()
                }
            })
        }
        DeriveData {
            what,
            name,
            generic,
            fields
        }
    }
}
impl std::fmt::Display for DeriveData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}\n", self.name)?;
        write!(f, "what: {}\n", self.what)?;
        write!(f, "generic: {:?}\n", self.generic.iter().map(|x| x.to_string()).collect::<Vec<String>>())?;
        write!(f, "fields: {:?}", self.fields)
    }
}
#[proc_macro_derive(Test)]
pub fn test(input: TokenStream) -> TokenStream {
    let mut out = String::new();
    printer(&input, 0, &mut out);
    let data = DeriveData::from(input);
    std::fs::write("token.txt", out).unwrap();
    std::fs::write("data.txt", data.to_string()).unwrap();
    std::fs::write("out.txt", data.implement(Which::From)).unwrap();
    TokenStream::new()
}
fn printer(input: &TokenStream, layer: usize, out: &mut String) {
    for i in input.clone().into_iter() {
        match i.clone() {
            TokenTree::Group(group) => {
                *out += &"\t".repeat(layer);
                *out += "group:\n";
                printer(&group.stream(), layer+1, out)
            }
            TokenTree::Ident(ident) => {
                *out += &"\t".repeat(layer);
                *out += "ident: ";
                *out += &ident.to_string();
                *out += "\n";
            }
            TokenTree::Literal(literal) => {
                *out += &"\t".repeat(layer);
                *out += "literal: ";
                *out += &literal.to_string();
                *out += "\n"
            }
            TokenTree::Punct(punct) => {
                *out += &"\t".repeat(layer);
                *out += "punct: ";
                *out += &punct.to_string();
                *out += "\n"
            }
        }
    }
}
#[derive(Debug)]
struct Field {
    name: String,
    data_type: String,
}
enum Which {
    From,
    To
}
#[proc_macro_derive(FromBinary)]
pub fn from_binary(input: TokenStream) -> TokenStream {
    DeriveData::from(input).implement(Which::From).parse::<TokenStream>().unwrap()
}
#[proc_macro_derive(ToBinary)]
pub fn to_binary(input: TokenStream) -> TokenStream {
    DeriveData::from(input).implement(Which::To).parse::<TokenStream>().unwrap()
}