extern crate proc_macro;

use proc_macro_error::{
    abort_call_site,
    proc_macro_error
};
use proc_macro::TokenStream;

#[proc_macro]
#[proc_macro_error]
pub fn contract(input: TokenStream) -> TokenStream {
    let contract_path = syn::parse_macro_input!(input as syn::LitStr);

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let metadata_path: std::path::PathBuf = [&root, &contract_path.value(), "target", "ink", "metadata.json"].iter().collect();

    let reader = std::fs::File::open(metadata_path)
        .unwrap_or_else(|e| abort_call_site!("Failed to read metadata file: {}", e));
    let metadata: ink_metadata::MetadataVersioned = serde_json::from_reader(reader)
        .unwrap_or_else(|e| abort_call_site!("Failed to deserialize metadata file: {}", e));

    if let ink_metadata::MetadataVersioned::V1(ink_project) = metadata {
        let contract_mod = generate_contract_mod(ink_project);
        contract_mod.into()
    } else {
        abort_call_site!("Invalid ink! metadata version: expected V1")
    }

}

// #[derive(Clone, Debug, Deserialize)]
// pub struct ContractMetadata {
//     #[serde(flatten)]
//     pub abi: ink_metadata::MetadataVersioned,
// }

fn generate_contract_mod(metadata: ink_metadata::InkProject) -> proc_macro2::TokenStream {
    quote::quote!()
}
