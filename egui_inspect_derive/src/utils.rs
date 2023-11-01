use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::Type::{Path, Reference};
use syn::{Field, Ident, Type};

use crate::AttributeArgs;

pub fn get_path_str(type_path: &Type) -> Option<String> {
    match type_path {
        Path(type_path) => {
            let ident = type_path.path.get_ident();
            if let Some(name) = ident {
                return Some(name.to_string());
            }
            return None;
        }
        Reference(type_ref) => get_path_str(&*type_ref.elem),
        _ => Some("".to_string()),
    }
}

pub(crate) fn get_default_function_call(
    name: &str,
    field: &TokenStream,
    mutable: bool,
) -> TokenStream {
    let ref_str = if mutable { quote!(&mut) } else { quote!(&) };
    let inspect = if mutable {
        quote!(inspect_mut)
    } else {
        quote!(inspect)
    };

    quote! { egui_inspect::EguiInspect::#inspect(#ref_str #field, #name, ui); }
}
