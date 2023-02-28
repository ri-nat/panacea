#![deny(clippy::unwrap_used, unsafe_code)]

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parser, punctuated::Punctuated, ItemFn, Path, Token};

extern crate panacea_types;
extern crate state;

#[proc_macro_attribute]
/// Used to mark a function as an event handler.
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as ItemFn);
    let original_fn = item.clone();
    let fn_name_ident = item.sig.ident;
    let struct_name_ident = prefix_ident("panacea_handler", &fn_name_ident);

    let mut state_vars = Vec::new();
    // Stores arguments for the function we'll call later
    let mut handle_fn_args: Punctuated<Ident, Token![,]> = Punctuated::new();
    // Collect arguments with type `&State<T>`
    for arg in item.sig.inputs.iter() {
        let syn::FnArg::Typed(syn::PatType { ty, pat, .. }) = arg else {
            continue;
        };

        let syn::Type::Reference(syn::TypeReference { elem, .. }) = ty.as_ref() else {
            continue;
        };

        let syn::Type::Path(syn::TypePath { path, .. }) = elem.as_ref() else {
            continue;
        };

        if path.segments.is_empty() {
            continue;
        }

        let first_segment = path
            .segments
            .first()
            .expect("path.segments is empty, but it shouldn't be.");

        // TODO: make it work for aliased types
        if first_segment.ident == "State" {
            let syn::Pat::Ident(syn::PatIdent { ident, .. }) = pat.as_ref() else {
                continue;
            };

            let state_var_ident = prefix_ident("panaces_state", ident);

            state_vars.push(quote! {
                let #state_var_ident = state.get::<#path>();
            });

            handle_fn_args.push(state_var_ident);
        }
    }

    quote! {
        #[allow(clippy::unnecessary_wraps)]
        #original_fn

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        struct #struct_name_ident;

        #[async_trait::async_trait]
        impl panacea_types::Handler for #struct_name_ident {
            #[cfg(feature = "mysql")]
            async fn handle(
                &self,
                state: &mut panacea_types::state::Container![Send + Sync],
                tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
                event: &panacea_types::Event,
            ) -> panacea_types::handler::HandlingResult {
                #(#state_vars)*

                #fn_name_ident(#handle_fn_args)
            }

            #[cfg(feature = "postgres")]
            async fn handle<'a>(
                &self,
                state: &mut panacea_types::state::Container![Send + Sync],
                tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
                event: &panacea_types::Event,
            ) -> panacea_types::handler::HandlingResult {
                #(#state_vars)*

                #fn_name_ident(#handle_fn_args)
            }

            #[cfg(feature = "sqlite")]
            async fn handle(
                &self,
                state: &mut panacea_types::state::Container![Send + Sync],
                tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
                event: &panacea_types::Event,
            ) -> panacea_types::handler::HandlingResult {
                #(#state_vars)*

                #fn_name_ident(#handle_fn_args)
            }
        }
    }
    .into()
}

/// Takes a coma-seperated list of function identifiers and generates vector with corresponding
/// handler structs. This is useful for resolving events to handlers.
///
/// Only works with functions that have been wrapped with the [`macro@handler`] attribute.
///
/// # Example
///
/// ```
/// use panacea_proc_macros::{handler, handlers};
/// use panacea_types::{Event, HandlingResult, MaybeHandlers};
///
/// #[handler]
/// fn send_welcome_email() -> HandlingResult {
///     Ok(None)
/// }
///
/// #[handler]
/// fn notify_admins() -> HandlingResult {
///     Ok(None)
/// }
///
/// // You can use this function when building a worker with `panacea::Worker::with_handlers_resolver()`.
/// fn resolver(event: &Event) -> MaybeHandlers {
///     match event.headers.get("event") {
///         Some(name) => match name.as_str() {
///             "user_created" => handlers![send_welcome_email, notify_admins],
///             _ => None,
///         },
///         None => None,
///     }
/// }
/// ```
#[proc_macro]
pub fn handlers(input: TokenStream) -> TokenStream {
    let paths = <Punctuated<Path, Token![,]>>::parse_terminated
        .parse(input)
        .expect("failed to parse input");
    let exprs = paths.iter().map(|path| {
        let expr = path.to_token_stream().to_string();
        let expr = expr.trim_matches('"');
        let expr = format!("Box::new(panacea_handler_{expr})");

        syn::parse_str::<syn::Expr>(&expr).expect("failed to parse expression")
    });

    quote!(Some(vec![#(#exprs),*])).into()
}

/// Takes a prefix and an identifier and generates a new identifier with the prefix, stripping an underscore from the identifier if it exists.
fn prefix_ident(prefix: &str, ident: &Ident) -> Ident {
    let ident = &ident.to_string();
    let ident = if let Some(stripped) = ident.strip_prefix('_') {
        stripped
    } else {
        ident
    };

    format_ident!("{}_{}", prefix, ident)
}
