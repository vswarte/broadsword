use syn::ItemFn;
use quote::quote;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn entrypoint(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn: ItemFn = syn::parse_macro_input!(input as ItemFn);
    let input_fn_ident = input_fn.sig.ident.clone();

    TokenStream::from(quote! {
        #input_fn

        #[no_mangle]
        pub extern "stdcall" fn DllMain(base: usize, reason: u32) -> bool {
            match reason {
                1 => { #input_fn_ident(base) }
                _ => true,
            }
        }
    })
}