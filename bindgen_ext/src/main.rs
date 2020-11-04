use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use std::fs::read_to_string;
use std::io::Write;
use syn;
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemStruct, LitInt, ReturnType, Type,
};
mod parse;
use parse::*;

fn main() {
    let bindings_rs = "bindgen_ext/src/simple.rs";
    assert!(std::path::Path::new(bindings_rs).exists());
    let source = read_to_string(bindings_rs).unwrap();
    let mut tree: syn::File = syn::parse_file(&source).expect("Could not parse source");
    let mut f = std::fs::File::create("/tmp/bla.rs").unwrap();

    let mut new_tree = vec![];
    let mut structs = parse::iter_structs(&tree.items);
    for _struct in structs {
        let wrap = wrapper_struct(_struct);
        new_tree.push(wrap);
        let mut getters = vec![];
        for fld in _struct.fields.iter() {
            let fld_ident = fld.ident.as_ref().unwrap();
            let method = method_name(fld_ident, MethodType::Getter);
            let rt = parse::return_type(&fld);
            let getter = parse::getter(&method, &fld.ident.as_ref().unwrap(), &rt);
            getters.push(getter);
        }
        let new_name = parse::renamed_struct(&_struct.ident);
        let _impl = parse::impl_struct(&new_name, &getters);
        new_tree.push(_impl);
    }
    for i in &new_tree {
        f.write_all(i.to_string().as_bytes());
    }
    std::process::Command::new("rustfmt")
        .arg("/tmp/bla.rs")
        .status()
        .unwrap();
}