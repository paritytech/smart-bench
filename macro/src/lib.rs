extern crate proc_macro;

use contract_metadata::ContractMetadata;
use heck::ToUpperCamelCase as _;
use ink_metadata::{InkProject, MetadataVersion, Selector};
use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use subxt_codegen::TypeGenerator;

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

    let reader = std::fs::File::open(std::path::Path::new(metadata_path.clone()))
        .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file: {}", e));
    let metadata: ContractMetadata = serde_json::from_reader(reader)
        .unwrap_or_else(|e| abort_call_site!("Failed to deserialize contract metadata: {}", e));

    let contract_name = metadata.contract.name;
    let version: MetadataVersion =
        serde_json::from_value(metadata.abi.get("version").expect("version").clone())
            .unwrap_or_else(|e| abort_call_site!("Failed to deserialize metadata file: {}", e));
    if version == MetadataVersion::V4 {
        let reader = std::fs::File::open(metadata_path.clone())
            .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file for version: {}", e));
        let ink_project: InkProject = serde_json::from_reader(reader)
            .unwrap_or_else(|e| abort_call_site!("Failed to deserialize contract metadata for version: {}", e));
        let contract_mod = generate_contract_mod(contract_name, ink_project);
        contract_mod.into()
    } else {
        // TODO better message
        abort_call_site!("Invalid contract metadata version")
    }
}

fn generate_contract_mod(contract_name: String, metadata: InkProject) -> proc_macro2::TokenStream {
    let type_substitutes = [(
        "ink_env::types::AccountId",
        syn::parse_quote!(::sp_core::crypto::AccountId32),
    )]
    .iter()
    .map(|(path, substitute): &(&str, syn::TypePath)| (path.to_string(), substitute.clone()))
    .collect();

    let type_generator = TypeGenerator::new(
        metadata.registry(),
        "contract_types",
        type_substitutes,
        Default::default(),
    );
    let types_mod = type_generator.generate_types_mod();
    let types_mod_ident = types_mod.ident();

    let contract_name = quote::format_ident!("{}", contract_name);
    let constructors = generate_constructors(&metadata, &type_generator);
    let messages = generate_messages(&metadata, &type_generator);

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
    let trait_path = syn::parse_quote!(ink_env::e2e::InkConstructor);
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
            generate_message_impl(type_gen, name, args, constructor.selector(), &trait_path)
        })
        .collect()
}

fn generate_messages(
    metadata: &ink_metadata::InkProject,
    type_gen: &TypeGenerator,
) -> Vec<proc_macro2::TokenStream> {
    let trait_path = syn::parse_quote!(ink_env::e2e::InkMessage);
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

            generate_message_impl(type_gen, name, args, message.selector(), &trait_path)
        })
        .collect()
}

fn generate_message_impl(
    type_gen: &TypeGenerator,
    name: &str,
    args: Vec<(&str, u32)>,
    selector: &Selector,
    impl_trait: &syn::Path,
) -> proc_macro2::TokenStream {
    let struct_ident = quote::format_ident!("{}", name.to_upper_camel_case());
    let fn_ident = quote::format_ident!("{}", name);
    let (args, arg_names): (Vec<_>, Vec<_>) = args
        .iter()
        .map(|(name, type_id)| {
            let name = quote::format_ident!("{}", name);
            let ty = type_gen.resolve_type_path(*type_id, &[]);
            (quote::quote!( #name: #ty ), name)
        })
        .unzip();
    let selector_bytes = hex_lits(selector);
    quote::quote! (
        #[derive(::scale::Encode)]
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
