extern crate proc_macro;

use contract_metadata::ContractMetadata;
use heck::ToUpperCamelCase as _;
use ink_metadata::{InkProject, MetadataVersion, Selector};
use proc_macro::TokenStream;
use std::path::PathBuf;
use proc_macro_error::{abort_call_site, proc_macro_error};
use subxt_codegen::{DerivesRegistry, TypeGenerator};
use syn::ReturnType;

#[proc_macro]
#[proc_macro_error]
pub fn contract(input: TokenStream) -> TokenStream {
    let contract_path = syn::parse_macro_input!(input as syn::LitStr);

    let metadata_path = contract_path.value();
    let metadata_path = std::path::PathBuf::from(metadata_path).canonicalize().expect("canonicalize must work");
    eprintln!("canonical metadata_path in `smart-bench-macro`: {:?}", metadata_path);

    std::path::Path::new(&metadata_path.clone())
        .try_exists()
        .unwrap_or_else(|err| {
            panic!("path does not exist: {:?}", err);
        });

    eprintln!("existing canonical metadata_path in `smart-bench-macro`: {:?}", metadata_path);

    let reader = std::fs::File::open(std::path::Path::new(&metadata_path.clone()))
        .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file: {}", e));
    let metadata: ContractMetadata = serde_json::from_reader(reader)
        .unwrap_or_else(|e| abort_call_site!("Failed to deserialize contract metadata: {}", e));

    let version: MetadataVersion =
        serde_json::from_value(metadata.abi.get("version").expect("version").clone())
            .unwrap_or_else(|e| abort_call_site!("Failed to deserialize metadata file: {}", e));
    if version == MetadataVersion::V4 {
        let reader = std::fs::File::open(metadata_path.clone())
            .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file for version: {}", e));
        let ink_project: InkProject = serde_json::from_reader(reader)
            .unwrap_or_else(|e| abort_call_site!("Failed to deserialize contract metadata for version: {}", e));
        let contract_mod = generate_contract_mod(metadata, ink_project, &metadata_path);
        contract_mod.into()
    } else {
        // TODO better message
        abort_call_site!("Invalid contract metadata version")
    }
}

fn generate_contract_mod(contract_metadata: ContractMetadata, metadata: InkProject, metadata_path: &PathBuf,) -> proc_macro2::TokenStream {
    let contract_name = contract_metadata.contract.name;
    let type_substitutes = [
        (
            "ink::env::types::AccountId",
            syn::parse_quote!(::sp_core::crypto::AccountId32),
        ),
        (
            //"ink::env::types::Hash",
            "ink_primitives::types::Hash",
            syn::parse_quote!(::ink::primitives::Hash),
            //syn::parse_quote!(::sp_core::H256),
        ),
    ]
    .iter()
    .map(|(path, substitute): &(&str, syn::TypePath)| (path.to_string(), substitute.clone()))
    .collect();

    //let crate_path: syn::Path = parse_quote!(::ink::env::e2e::subxt);
    let crate_path = String::from("::ink_e2e::subxt").into();
    let type_generator = TypeGenerator::new(
        metadata.registry(),
        "contract_types",
        //"ink::primitives",
        type_substitutes,
        //DerivesRegistry::default_with_crate_path(&crate_path),
        DerivesRegistry::new(&crate_path),
        crate_path,
    );
    //let types_mod = type_generator.generate_types_mod(&crate_path.into());
    //let path = String::from("::ink::env::e2e::subxt");
    let types_mod = type_generator.generate_types_mod();
    let types_mod_ident = types_mod.ident();

    let contract_name = quote::format_ident!("{}", contract_name);
    let constructors = generate_constructors(&metadata, &type_generator, &metadata_path);
    let messages = generate_messages(&metadata, &type_generator, &metadata_path);

    let path = metadata_path.clone().into_os_string().into_string().expect("conversion failed");

    quote::quote!(
        pub mod #contract_name {
            //pub const PATH: &'static str = "target/ink/accumulator/accumulator.contract";
    pub const CONTRACT_PATH: &'static str = #path;
            #types_mod

            //pub const _ink_contract_path: &str = "target/ink/accumulator/accumulator.contract";
            //pub const _ink_hash: Hash = ;

            pub mod constructors {
                use super::#types_mod_ident;
                #( #constructors )*
            }

            pub mod messages {
                use super::#types_mod_ident;
                #( #messages )*
            }
        }

        //impl ::ink_e2e::Contract for #contract_name {
            //const PATH: &'static str = "target/ink/accumulator/accumulator.contract";
        //}
    )
}

fn generate_constructors(
    metadata: &ink_metadata::InkProject,
    type_gen: &TypeGenerator,
    metadata_path: &PathBuf,
) -> Vec<proc_macro2::TokenStream> {
    let trait_path = syn::parse_quote!(::ink_e2e::InkConstructor);
    metadata
        .spec()
        .constructors()
        .iter()
        .map(|constructor| {
            let name = constructor.label();
            let args = constructor
                .args()
                .iter()
                .map(|arg| (arg.label().as_str(), arg.ty().ty().id()))
                .collect::<Vec<_>>();

            //let return_type = constructor.return_type().opt_type().unwrap();
            //let return_type: String = return_type.display_name().segments().join("::");
            //let return_type = Some("Self"); //&String::from("Self"));
            let return_type = Some(String::from("Self"));
            let return_type = None;
            /*
            let return_type = message.return_type().opt_type().map(|return_type| {
                return_type.display_name().segments().join("::")
            });
             */
            generate_message_impl(type_gen, name, args, constructor.selector(), &trait_path, metadata_path, return_type)
        })
        .collect()
}

fn generate_messages(
    metadata: &ink_metadata::InkProject,
    type_gen: &TypeGenerator,
    metadata_path: &PathBuf,
) -> Vec<proc_macro2::TokenStream> {
    let trait_path = syn::parse_quote!(::ink_e2e::InkMessage);
    metadata
        .spec()
        .messages()
        .iter()
        .map(|message| {
            // strip trait prefix from trait message labels
            let name =
                message.label().split("::").last().unwrap_or_else(|| {
                    abort_call_site!("Invalid message label: {}", message.label())
                });
            let args = message
                .args()
                .iter()
                .map(|arg| (arg.label().as_str(), arg.ty().ty().id()))
                .collect::<Vec<_>>();

            /*
            let return_type = match message.return_type().opt_type() {
                Some(return_type) => return_type.display_name().segments().join("::"),
                None => String::from("bool"),
            };
             */
            let return_type = message.return_type().opt_type().map(|return_type| {
                return_type.display_name().segments().join("::")
            });
            //let return_type = return_type.ty().into();

            //eprintln!("args {:?}", args);
            generate_message_impl(type_gen, name, args, message.selector(), &trait_path, metadata_path, return_type)
        })
        .collect()
}

fn generate_message_impl(
    type_gen: &TypeGenerator,
    name: &str,
    args: Vec<(&str, u32)>,
    selector: &Selector,
    impl_trait: &syn::Path,
    metadata_path: &PathBuf,
    //return_type: &syn::Path,
    return_type: Option<String>,
) -> proc_macro2::TokenStream {
    /*
    let return_type = return_type.map(|return_type| {
        syn::parse_str::<syn::Path>(&return_type).expect("oh no path")
    });
     */
    let return_type: syn::Type = match return_type {
        Some(return_type) => {
            syn::parse_str::<syn::Type>(&return_type).expect("oh no path")
            //syn::parse_quote!( #return_type )
            //ReturnType::parse(return_type)
            //quote::quote!( #return_type )
        },
        None => {
            syn::parse_str::<syn::Type>("()").expect("oh no path")
            //syn::parse_quote!( () )
            //ReturnType::Default
        }
        //None =>  syn::parse_str::<syn::ReturnType>("()").expect("oh no ()"),
        //None =>  ReturnType::Default,
    };
    let struct_ident = quote::format_ident!("{}", name.to_upper_camel_case());
    //eprintln!("\nstruct_ident {:?}", struct_ident);
    let fn_ident = quote::format_ident!("{}", name);
    let (args, arg_names): (Vec<_>, Vec<_>) = args
        .iter()
        .map(|(name, type_id)| {
            let name = quote::format_ident!("{}", name);
            //let ty = type_gen.resolve_type_path(*type_id, &[]);
            let ty = type_gen.resolve_type_path(*type_id);
            (quote::quote!( #name: #ty ), name)
        })
        .unzip();
    let selector_bytes = hex_lits(selector);
    let path = metadata_path.clone().into_os_string().into_string().expect("conversion failed");
    //eprintln!("arg_names {:?}", arg_names);
    quote::quote! (
        #[derive(::scale::Encode)]
        pub struct #struct_ident {
            #( #args ), *
        }

        impl #impl_trait for #struct_ident {
            type ReturnType = #return_type;
            const SELECTOR: [u8; 4] = [ #( #selector_bytes ),* ];
            const CONTRACT_PATH: &'static str = #path;
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
    let hex_lits = selector
        .to_bytes()
        .iter()
        .map(|byte| {
            syn::LitInt::new(
                &format!("0x{:02X}_u8", byte),
                proc_macro2::Span::call_site(),
            )
        })
        .collect::<Vec<_>>();
    hex_lits.try_into().expect("Invalid selector bytes length")
}
