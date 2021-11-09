extern crate proc_macro;

use heck::CamelCase;
use proc_macro::TokenStream;
use ink_metadata::Selector;
use proc_macro_error::{abort_call_site, proc_macro_error};
use serde::Deserialize;
use subxt_codegen::TypeGenerator;

#[proc_macro]
#[proc_macro_error]
pub fn contract(input: TokenStream) -> TokenStream {
    let contract_path = syn::parse_macro_input!(input as syn::LitStr);

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let metadata_path: std::path::PathBuf = [
        &root,
        &contract_path.value(),
        "target",
        "ink",
        "metadata.json",
    ]
    .iter()
    .collect();

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
    let type_generator = TypeGenerator::new(
        metadata.v1.registry(),
        "contract_types",
        Default::default(),
        Default::default(),
    );
    let types_mod = type_generator.generate_types_mod();
    let types_mod_ident = types_mod.ident();

    let contract_name = quote::format_ident!("{}", metadata.contract.name);
    let constructors = generate_constructors(&metadata.v1, &type_generator);
    let messages = generate_messages(&metadata.v1, &type_generator);

    quote::quote!(
        pub mod #contract_name {
            #types_mod

            pub mod constructors {
                use super::#types_mod_ident;
                #( #constructors )*
            }

            pub mod messages {
                use super::#types_mod_ident;
                #( #messages )*
            }
        }
    )
}

fn generate_constructors(
    metadata: &ink_metadata::InkProject,
    type_gen: &TypeGenerator,
) -> Vec<proc_macro2::TokenStream> {
    let trait_path = syn::parse_quote!(crate::InkConstructor);
    metadata
        .spec()
        .constructors()
        .iter()
        .map(|constructor| {
            let name = constructor
                .name()
                .last()
                .expect("Constructor should have a name");
            let args = constructor.args().iter().map(|arg| {
                (arg.name().as_str(), arg.ty().ty().id())
            }).collect::<Vec<_>>();
            generate_message_impl(type_gen, name, args, constructor.selector(), &trait_path)

        })
        .collect()
}

fn generate_messages(
    metadata: &ink_metadata::InkProject,
    type_gen: &TypeGenerator,
) -> Vec<proc_macro2::TokenStream> {
    let trait_path = syn::parse_quote!(crate::InkMessage);
    metadata
        .spec()
        .messages()
        .iter()
        .map(|message| {
            let name = message.name().last().expect("Message should have a name");
            let args = message.args().iter().map(|arg| {
                (arg.name().as_str(), arg.ty().ty().id())
            }).collect::<Vec<_>>();

            generate_message_impl(type_gen, name, args, message.selector(), &trait_path)
        })
        .collect()
}

fn generate_message_impl(type_gen: &TypeGenerator, name: &str, args: Vec<(&str, u32)>, selector: &Selector, impl_trait: &syn::Path) -> proc_macro2::TokenStream {
    let struct_ident = quote::format_ident!("{}", name.to_camel_case());
    let fn_ident = quote::format_ident!("{}", name);
    let (args, arg_names): (Vec<_>, Vec<_>) =args.iter().map(|(name, type_id)| {
        let name = quote::format_ident!("{}", name);
        let ty = type_gen.resolve_type_path(*type_id, &[]);
        (quote::quote!( #name: #ty ), name)
    }).unzip();
    let selector_bytes = hex_lits(selector);
    quote::quote! (
        #[derive(::codec::Encode)]
        pub struct #struct_ident {
            #( #args ), *
        }

        impl #impl_trait for #struct_ident {
            const SELECTOR: [u8; 4] = [ #( #selector_bytes ),* ];
        }

        pub fn #fn_ident(#( #args ), *) -> #struct_ident {
            #struct_ident {
                #( #arg_names ), *
            }
        }
    )
}

/// Returns the 4 bytes that make up the selector as hex encoded bytes.
fn hex_lits(selector: &ink_metadata::Selector) -> [syn::LitInt; 4] {
    let hex_lits =
        selector.to_bytes().iter().map(|byte|
            syn::LitInt::new(
                &format!("0x{:02X}_u8", byte),
                proc_macro2::Span::call_site()
            )
        ).collect::<Vec<_>>();
    hex_lits.try_into().expect("Invalid selector bytes length")
}
