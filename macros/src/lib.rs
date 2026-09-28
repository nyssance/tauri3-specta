//! Macros connecting one command declaration to Tauri and Specta.

use heck::ToKebabCase;
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote};
use syn::{DeriveInput, ItemFn, LitStr, Path, Token, parse_macro_input, punctuated::Punctuated};

fn library() -> syn::Result<Tokens> {
    match crate_name("tauri3-specta").map_err(|error| syn::Error::new(Span::call_site(), error))? {
        FoundCrate::Itself => Ok(quote!(::tauri3_specta)),
        FoundCrate::Name(name) => {
            let name = format_ident!("{}", name);
            Ok(quote!(::#name))
        }
    }
}

/// Registers a command with both Tauri and Specta.
/// Supports Tauri's `rename` and `rename_all` options.
#[proc_macro_attribute]
pub fn command(attributes: TokenStream, input: TokenStream) -> TokenStream {
    let function = parse_macro_input!(input as ItemFn);
    match expand_command(attributes.into(), function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_command(attributes: Tokens, function: ItemFn) -> syn::Result<Tokens> {
    let root = library()?;
    let mut name = function
        .sig
        .ident
        .to_string()
        .trim_start_matches("r#")
        .to_owned();
    let mut snake_case = false;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("rename") {
            name = meta.value()?.parse::<LitStr>()?.value();
        } else if meta.path.is_ident("rename_all") {
            let value = meta.value()?.parse::<LitStr>()?;
            snake_case = match value.value().as_str() {
                "snake_case" => true,
                "camelCase" => false,
                _ => return Err(meta.error("expected snake_case or camelCase")),
            };
        } else {
            return Err(meta.error("supported command options: rename, rename_all"));
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, attributes.clone())?;
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &function.sig.generics,
            "use concrete Tauri 3 command signatures; AppHandle defaults to DynRuntime",
        ));
    }
    let identifier = &function.sig.ident;
    let helper = format_ident!(
        "__tauri3_specta_{}",
        identifier.to_string().trim_start_matches("r#")
    );
    let visibility = &function.vis;
    let conditional_attributes: Vec<_> = function
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .collect();
    Ok(quote! {
        #[::tauri::command(#attributes)]
        #[::specta::specta]
        #function

        #(#conditional_attributes)*
        #[doc(hidden)]
        #visibility fn #helper(types: &mut ::specta::Types) -> #root::__private::Command {
            #root::__private::Command::new(
                ::specta::function::fn_datatype!(#identifier)(types), #name, #snake_case,
            )
        }
    })
}

/// Collects the same commands into the IPC handler and the binding schema.
#[proc_macro]
pub fn commands(input: TokenStream) -> TokenStream {
    let paths = parse_macro_input!(input with Punctuated::<Path, Token![,]>::parse_terminated);
    let root = match library() {
        Ok(root) => root,
        Err(error) => return error.into_compile_error().into(),
    };
    let paths: Vec<_> = paths.into_iter().collect();
    let helpers: Vec<_> = paths
        .iter()
        .map(|path| {
            let mut helper = path.clone();
            let last = helper
                .segments
                .last_mut()
                .expect("command path is nonempty");
            last.ident = format_ident!(
                "__tauri3_specta_{}",
                last.ident.to_string().trim_start_matches("r#")
            );
            helper
        })
        .collect();
    quote! {
        #root::Bindings::from_commands(
            ::tauri::generate_handler![#(#paths),*],
            |types| vec![#(#helpers(types)),*],
        )
    }
    .into()
}

/// Derives a shared Rust/TypeScript event name; override with `#[event(name = "...")]`.
#[proc_macro_derive(Event, attributes(event))]
pub fn event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_event(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_event(input: DeriveInput) -> syn::Result<Tokens> {
    let root = library()?;
    let identifier = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            input.generics,
            "events require a concrete payload type",
        ));
    }
    let mut name = identifier.to_string().to_kebab_case();
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("event"))
    {
        attribute.parse_nested_meta(|meta| {
            if !meta.path.is_ident("name") {
                return Err(meta.error("expected name"));
            }
            name = meta.value()?.parse::<LitStr>()?.value();
            Ok(())
        })?;
    }
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-/:_".contains(character))
    {
        return Err(syn::Error::new_spanned(
            identifier,
            "event names may contain ASCII letters, digits, -, /, :, _",
        ));
    }
    Ok(quote! {
        impl #root::Event for #identifier {
            const NAME: &'static str = #name;
        }
    })
}
