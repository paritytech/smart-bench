extern crate proc_macro;

use proc_macro_error::proc_macro_error;
use proc_macro::TokenStream;

#[proc_macro]
#[proc_macro_error]
pub fn contract(input: TokenStream) -> TokenStream {
    let metadata_path = syn::parse_macro_input!(input as syn::LitStr);
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let root_path = std::path::Path::new(&root);
    let path = root_path.join(metadata_path.value());

    println!("PATH {}", path.to_string_lossy());

    let tokens = quote::quote! ();

    tokens.into()
}
