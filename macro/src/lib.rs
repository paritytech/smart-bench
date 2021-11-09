extern crate proc_macro;

use proc_macro_error::{
    abort_call_site,
    proc_macro_error
};
use proc_macro::TokenStream;
use serde::Deserialize;

#[proc_macro]
#[proc_macro_error]
pub fn contract(input: TokenStream) -> TokenStream {
    let contract_path = syn::parse_macro_input!(input as syn::LitStr);

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let metadata_path: std::path::PathBuf = [&root, &contract_path.value(), "target", "ink", "metadata.json"].iter().collect();

    let reader = std::fs::File::open(metadata_path)
        .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file: {}", e));
    let metadata: ContractMetadata = serde_json::from_reader(reader)
        .unwrap_or_else(|e| abort_call_site!("Failed to deserialize metadata file: {}", e));

    let contract_mod = generate_contract_mod(metadata);
    contract_mod.into()
}

#[derive(Deserialize)]
struct ContractMetadata {
    contract: Contract,
    #[serde(rename = "V1")]
    pub v1: ink_metadata::InkProject,
}

/// Metadata about a smart contract.
#[derive(Deserialize)]
struct Contract {
    name: String,
}

fn generate_contract_mod(metadata: ContractMetadata) -> proc_macro2::TokenStream {
    let contract_name = quote::format_ident!("{}", metadata.contract.name);
    let constructors = generate_constructors(&metadata.v1);
    let messages = generate_messages(&metadata.v1);

    quote::quote!(
        pub mod #contract_name {
            pub mod constructors {
                #( #constructors )*
            }

            pub mod messages {
                #( #messages )*
            }
        }
    )
}

fn generate_constructors(metadata: &ink_metadata::InkProject) -> Vec<proc_macro2::TokenStream> {
    metadata
        .spec()
        .constructors()
        .iter()
        .map(|constructor| {
            let name = &constructor.name().last().expect("Message should have a name");
            let fn_ident = quote::format_ident!("{}", name);
            let args = constructor
                .args
                .iter()
                .map(|arg| {
                    let name = quote::format_ident!("{}", arg.name());
                    let ty = quote::quote!( u8 ); // todo
                    quote::quote!( #name: #ty )
                });
            quote::quote! (
                pub fn #fn_ident(#( #args ), *) -> ::std::vec::Vec<u8> {
                    todo!()
                }
            )
        })
        .collect()
}

fn generate_messages(metadata: &ink_metadata::InkProject) -> Vec<proc_macro2::TokenStream> {
    metadata
        .spec()
        .messages()
        .iter()
        .map(|message| {
            let name = &message.name().last().expect("Message should have a name");
            let fn_ident = quote::format_ident!("{}", name);
            let args = message
                .args()
                .iter()
                .map(|arg| {
                    let name = quote::format_ident!("{}", arg.name());
                    let ty = quote::quote!( u8 ); // todo
                    quote::quote!( #name: #ty )
                });
            quote::quote! (
                pub fn #fn_ident(#( #args ), *) -> ::std::vec::Vec<u8> {
                    todo!()
                }
            )
        })
        .collect()
}
