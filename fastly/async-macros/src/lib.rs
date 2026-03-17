use proc_macro::TokenStream;
use proc_macro2::Span;

/// Wraps an async main function as a Fastly Compute entry point.
///
/// # Example
///
/// ```rust,ignore
/// #[fastly_async::main]
/// async fn main(req: Request) -> Response {
///     // handle request
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;

    // Ensure the function is async.
    if input.sig.asyncness.is_none() {
        return syn::Error::new(Span::call_site(), "#[main] function must be async")
            .to_compile_error()
            .into();
    }

    // Ensure the function is not generic.
    if !input.sig.generics.params.is_empty() {
        return syn::Error::new(Span::call_site(), "#[main] function cannot be generic")
            .to_compile_error()
            .into();
    }

    quote::quote! {
        fn main() {
            async fn #fn_name(#fn_inputs) #fn_output #fn_block

            let req = ::fastly::Request::from_client();
            let resp = ::fastly_async::task::block_on(#fn_name(req));
            resp.send_to_client();
        }
    }
    .into()
}
