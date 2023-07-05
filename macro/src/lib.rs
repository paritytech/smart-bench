extern crate proc_macro;

use contract_metadata::ContractMetadata;
use heck::ToUpperCamelCase as _;
use ink_metadata::{InkProject, MetadataVersion, Selector};
use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use subxt_codegen::{CratePath, DerivesRegistry, TypeGenerator, TypeSubstitutes};

#[proc_macro]
#[proc_macro_error]
pub fn contract(input: TokenStream) -> TokenStream {
    let contract_path = syn::parse_macro_input!(input as syn::LitStr);

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let metadata_path: std::path::PathBuf = [&root, &contract_path.value()].iter().collect();

    let reader = std::fs::File::open(metadata_path)
        .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file: {}", e));
    let metadata: ContractMetadata = serde_json::from_reader(reader)
        .unwrap_or_else(|e| abort_call_site!("Failed to deserialize contract metadata: {}", e));
    let contract_name = metadata.contract.name;
    let ink_metadata: InkProject = serde_json::from_value(serde_json::Value::Object(metadata.abi))
        .unwrap_or_else(|e| abort_call_site!("Failed to deserialize ink metadata: {}", e));
    if &MetadataVersion::V4 == ink_metadata.version() {
        let contract_mod = generate_contract_mod(contract_name, ink_metadata);
        contract_mod.into()
    } else {
        abort_call_site!("Invalid contract metadata version")
    }
}

fn generate_contract_mod(contract_name: String, metadata: InkProject) -> proc_macro2::TokenStream {
    let crate_path = CratePath::default();
    let mut type_substitutes = TypeSubstitutes::new(&crate_path);

    let path_account: syn::Path = syn::parse_quote!(#crate_path::utils::AccountId32);
    let path_u256: syn::Path = syn::parse_quote!(::primitive_types::U256);

    type_substitutes
        .insert(
            syn::parse_quote!(ink_primitives::types::AccountId),
            path_account.try_into().unwrap(),
        )
        .expect("Error in type substitutions");

    type_substitutes
        .insert(
            syn::parse_quote!(ink_env::types::U256),
            path_u256.try_into().unwrap(),
        )
        .expect("Error in type substitutions");

    let type_generator = TypeGenerator::new(
        metadata.registry(),
        "contract_types",
        type_substitutes,
        DerivesRegistry::new(&crate_path),
        crate_path,
        false,
    );
    let types_mod = type_generator
        .generate_types_mod()
        .expect("Error in type generation");
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
    let trait_path = syn::parse_quote!(crate::InkConstructor);
    metadata
        .spec()
        .constructors()
        .iter()
        .map(|constructor| {
            let name = constructor.label();
            let args = constructor
                .args()
                .iter()
                .map(|arg| (arg.label().as_str(), arg.ty().ty().id))
                .collect::<Vec<_>>();
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
            // strip trait prefix from trait message labels
            let name =
                message.label().split("::").last().unwrap_or_else(|| {
                    abort_call_site!("Invalid message label: {}", message.label())
                });
            let args = message
                .args()
                .iter()
                .map(|arg| (arg.label().as_str(), arg.ty().ty().id))
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
        .enumerate()
        .map(|(i, (name, type_id))| {
            // In Solidity, function arguments may not have names.
            // If an argument without a name is included in the metadata, a name is generated for it
            let name = if name.is_empty() {
                format!("arg{i}")
            } else {
                name.to_string()
            };
            let name = quote::format_ident!("{}", name.as_str());
            let ty = type_gen.resolve_type_path(*type_id);
            (quote::quote!( #name: #ty ), name)
        })
        .unzip();
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

#[cfg(test)]
mod tests {
    use super::*;
    use ink_metadata::{
        layout::{Layout, StructLayout},
        ConstructorSpec, ContractSpec, MessageParamSpec, MessageSpec, ReturnTypeSpec, TypeSpec,
    };
    use ink_primitives::AccountId;
    use scale_info::{IntoPortable, Registry};

    // Helper for creating a InkProject with custom MessageSpec
    fn ink_project_with_custom_message(message: MessageSpec) -> InkProject {
        let mut registry = Registry::default();
        let spec = ContractSpec::new()
            .constructors([ConstructorSpec::from_label("New")
                .selector(Default::default())
                .payable(true)
                .args(Vec::new())
                .docs(Vec::new())
                .returns(ReturnTypeSpec::new(None))
                .done()])
            .messages([message])
            .docs(Vec::new())
            .done()
            .into_portable(&mut registry);
        let layout =
            Layout::Struct(StructLayout::new("Struct", Vec::new())).into_portable(&mut registry);
        InkProject::new_portable(layout, spec, registry.into())
    }

    #[test]
    fn test_contract_mod_with_ink_types_success() {
        let message = MessageSpec::from_label("set")
            .selector(Default::default())
            .mutates(false)
            .payable(true)
            .args(vec![MessageParamSpec::new("to")
                .of_type(TypeSpec::with_name_segs::<AccountId, _>(
                    ::core::iter::Iterator::map(
                        ::core::iter::IntoIterator::into_iter(["AccountId"]),
                        ::core::convert::AsRef::as_ref,
                    ),
                ))
                .done()])
            .returns(ReturnTypeSpec::new(None))
            .docs(Vec::new())
            .done();
        let metadata = ink_project_with_custom_message(message);
        let expected_output = quote::quote!(
            pub mod Test {
                pub mod contract_types {
                    use super::contract_types;
                }
                pub mod constructors {
                    use super::contract_types;
                    #[derive(:: codec :: Encode)]
                    pub struct New {}
                    impl crate::InkConstructor for New {
                        const SELECTOR: [u8; 4] = [0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8];
                    }
                    pub fn New() -> New {
                        New {}
                    }
                }
                pub mod messages {
                    use super::contract_types;
                    #[derive(:: codec :: Encode)]
                    pub struct Set {
                        to: ::subxt::utils::AccountId32
                    }
                    impl crate::InkMessage for Set {
                        const SELECTOR: [u8; 4] = [0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8];
                    }
                    pub fn set(to: ::subxt::utils::AccountId32) -> Set {
                        Set { to }
                    }
                }
            }
        );

        let generated_output = generate_contract_mod("Test".to_string(), metadata).to_string();
        assert_eq!(generated_output, expected_output.to_string())
    }
}
