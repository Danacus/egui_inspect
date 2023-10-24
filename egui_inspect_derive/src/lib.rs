use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, Field, Fields, FieldsNamed,
    GenericParam, Generics, Index, Variant, FieldsUnnamed,
};

use darling::{FromField, FromMeta};

mod internal_paths;
mod utils;

#[derive(Debug, FromField)]
#[darling(attributes(inspect), default)]
struct AttributeArgs {
    /// Name of the field to be displayed on UI labels
    name: Option<String>,
    /// Doesn't generate code for the given field
    hide: bool,
    /// Doesn't call mut function for the given field (May be overridden by other params)
    no_edit: bool,
    /// Use slider function for numbers
    slider: bool,
    /// Min value for numbers
    min: f32,
    /// Max value for numbers
    max: f32,
    /// Display mut text on multiple line
    multiline: bool,
    /// Use custom function for non-mut inspect
    custom_func: Option<String>,
    /// Use custom function for mut inspect
    custom_func_mut: Option<String>,
}

impl Default for AttributeArgs {
    fn default() -> Self {
        Self {
            name: None,
            hide: false,
            no_edit: false,
            slider: true,
            min: 0.0,
            max: 100.0,
            multiline: false,
            custom_func: None,
            custom_func_mut: None,
        }
    }
}

#[proc_macro_derive(EguiInspect, attributes(inspect))]
pub fn derive_egui_inspect(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let inspect = inspect_struct(&input.data, &name, false);

    let inspect_mut = inspect_struct(&input.data, &name, true);

    let expanded = quote! {
        impl #impl_generics egui_inspect::EguiInspect for #name #ty_generics #where_clause {
            fn inspect(&self, label: &str, ui: &mut egui::Ui) {
                #inspect
            }
            fn inspect_mut(&mut self, label: &str, ui: &mut egui::Ui) {
                #inspect_mut
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(egui_inspect::EguiInspect));
        }
    }
    generics
}

fn inspect_struct(data: &Data, _struct_name: &Ident, mutable: bool) -> TokenStream {
    match *data {
        Data::Struct(ref data) => handle_fields(&data.fields, mutable),
        Data::Enum(ref data_enum) => handle_enum(data_enum, _struct_name, mutable),
        Data::Union(_) => unimplemented!("Unions are not yet supported"),
    }
}

fn handle_enum(data_enum: &DataEnum, _struct_name: &Ident, mutable: bool) -> TokenStream {
    let variants: Vec<_> = data_enum.variants.iter().collect();
    let name_arms = variants.iter().map(|v| variant_name_arm(v, _struct_name));
    let reflect_variant_name = quote!(
        let current_variant = match self {
            #(#name_arms,)*
        };
);
    if mutable {
        let combo_opts = variants.iter().map(|v| variant_combo(v, _struct_name));
        let inspect_arms = variants.iter().map(|v| variant_inspect_arm(v, _struct_name));
        quote!(
            #reflect_variant_name
            ui.horizontal(|ui| {
                ui.label(stringify!(#_struct_name));
                ::egui::ComboBox::new(label, "")
                    .selected_text(current_variant)
                    .show_ui(ui, |ui| {
                        #(#combo_opts;)*
                    });
            });
            match self {
                #(#inspect_arms),*
            };
        )
    } else { 
        quote!(
            #reflect_variant_name
            ui.label(format!("{label}: {current_variant}").as_str());
            // TODO: readonly held data inspect
        )
    }
}

fn variant_name_arm(variant: &Variant, _struct_name: &Ident) -> TokenStream {
    let ident = &variant.ident;
    match &variant.fields {
        Fields::Named(_) => {
            quote!(#_struct_name::#ident {..} => stringify!(#ident))
        }
        Fields::Unnamed(_) => {
            quote!(#_struct_name::#ident (..) => stringify!(#ident))
        }
        Fields::Unit => {
            quote!(#_struct_name::#ident => stringify!(#ident))
        }
    }
}

fn variant_combo(variant: &Variant, _struct_name: &Ident) -> TokenStream {
    let ident = &variant.ident;
    // TODO: Replace with handle_fields,
    // which would need to take this ident as the base for fields instead of "self".
    match &variant.fields {
        Fields::Named(fields) => {
            let defaults = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.clone();
                    quote!( #ident: Default::default() )}
                );
            quote!(ui.selectable_value(self, 
                                       #_struct_name::#ident { #(#defaults),* }, 
                                       stringify!(#ident)))
        }
        Fields::Unnamed(fields) => {
            let defaults = fields
                .unnamed
                .iter()
                .map(|_| quote!(Default::default()));
            quote!(ui.selectable_value(self, #_struct_name::#ident ( #(#defaults),* ), stringify!(#ident)))
        }
        Fields::Unit => {
            quote!(ui.selectable_value(self, #_struct_name::#ident, stringify!(#ident)))
        }
    }
}

fn variant_inspect_arm(variant: &Variant, _struct_name: &Ident) -> TokenStream {
    let ident = &variant.ident;
    match &variant.fields {
        Fields::Named(fields) => {
            let field_idents: Vec<_> = fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.clone();
                    quote!( #ident )}
                ).collect();
            // TODO: properly refer to trait
            let inspect_fields = field_idents.iter().map(|f| quote!(#f.inspect_mut(stringify!(#f), ui)));
            quote!(#_struct_name::#ident { #(#field_idents),* } => { #(#inspect_fields;)* })
        }
        Fields::Unnamed(_) => {
            unimplemented!("TODO: unnamed")
        }
        Fields::Unit => {
            quote!(#_struct_name::#ident => () )
        }
    }
}

fn handle_fields(fields: &Fields, mutable: bool) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => handle_named_fields(fields, mutable),
        Fields::Unnamed(ref fields) => handle_unnamed_fields(fields, mutable), 
        // Empty implementation for unit fields (needed in plain enum variant for instance)
        Fields::Unit => quote!(),
    }
}

fn handle_named_fields(fields: &FieldsNamed, mutable: bool) -> TokenStream {
    let recurse = fields.named.iter().map(|f| {
        let attr = AttributeArgs::from_field(f).expect("Could not get attributes from field");

        if attr.hide {
            return quote!();
        }

        let mutable = mutable && !attr.no_edit;

        if let Some(ts) = handle_custom_func(&f, mutable, &attr) {
            return ts;
        }

        if let Some(ts) = internal_paths::try_handle_internal_path(&f, mutable, &attr) {
            return ts;
        }

        return utils::get_default_function_call(&f, mutable, &attr);
    });
    quote! {
        ui.strong(label);
        #(#recurse)*
    }
}

fn handle_unnamed_fields(fields: &FieldsUnnamed, mutable: bool) -> TokenStream {
    let mut recurse = Vec::new();
    for (i, _) in fields.unnamed.iter().enumerate() {
        let tuple_index = Index::from(i);
        let name = format!("Field {i}");
        let ref_str = if mutable { quote!(&mut) } else { quote!(&) };
        recurse.push(
            quote! { egui_inspect::EguiInspect::inspect(#ref_str self.#tuple_index, #name, ui);},
        );
    }

    let result = quote! {
        ui.strong(label);
        #(#recurse)*
    };
    result
}

fn handle_custom_func(field: &Field, mutable: bool, attrs: &AttributeArgs) -> Option<TokenStream> {
    let name = &field.ident;

    let name_str = match &attrs.name {
        Some(n) => n.clone(),
        None => name.clone().unwrap().to_string(),
    };

    if mutable && !attrs.no_edit && attrs.custom_func_mut.is_some() {
        let custom_func_mut = attrs.custom_func_mut.as_ref().unwrap();
        let ident = syn::Path::from_string(custom_func_mut)
            .expect(format!("Could not find function: {}", custom_func_mut).as_str());
        return Some(quote_spanned! { field.span() => {
                #ident(&mut self.#name, &#name_str, ui);
            }
        });
    }

    if (!mutable || (mutable && attrs.no_edit)) && attrs.custom_func.is_some() {
        let custom_func = attrs.custom_func.as_ref().unwrap();
        let ident = syn::Path::from_string(custom_func)
            .expect(format!("Could not find function: {}", custom_func).as_str());
        return Some(quote_spanned! { field.span() => {
                #ident(&self.#name, &#name_str, ui);
            }
        });
    }

    return None;
}
