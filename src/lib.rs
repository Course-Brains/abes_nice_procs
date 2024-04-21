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
pub fn method(attr: TokenStream) -> TokenStream {
    // Getting path and code
    let mut trees = attr.into_iter();
    let path = if let TokenTree::Ident(ident) = trees.next().unwrap() {
        ident.to_string()
    } else {
        panic!("could not get path")
    };

    let is_comma = matches!(trees.next(), Some(TokenTree::Punct(p)) if p.as_char() == ',');
    if !is_comma {
        panic!("expected comma after filename");
    }

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
        .spawn()
        .and_then(|mut c| c.wait())
        .expect("failed to compile");
    if !compile_status.success() {
        panic!("failed to compile: {compile_status}")
    }

    let bin_path = Path::new(".").join(&path);
    let _bin_path_guard = DeleteOnDrop::new(&bin_path);

    let output = std::process::Command::new(&bin_path).output().expect("failed to run file");
    if !output.status.success() {
        panic!("failed to run file: {}", output.status);
    }

    
    String::from_utf8(output.stdout)
        .unwrap()
        .parse::<TokenStream>()
        .unwrap()
}