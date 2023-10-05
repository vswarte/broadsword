use syn::{parse_macro_input, Data};

extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

#[proc_macro_attribute]
pub fn vftable(
    _args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let parsed = parse_macro_input!(item as syn::ItemStruct);
    let vftable_impl = build_impl(parsed.fields);

    let ident = &parsed.ident;
    proc_macro::TokenStream::from(quote! {
        impl #ident {
            #(#vftable_impl)*
        }
    })
}

fn build_impl(fields: syn::Fields) -> Vec<proc_macro2::TokenStream> {
    fields.iter()
        .map(|f| match &f.ty {
            syn::Type::BareFn(_fp) => {
                let ident = f.ident.as_ref()
                    .expect("vftable cannot contain anonymouse field");

                quote! {
                    fn #ident() {
                        println!("Lmao");
                    }
                }
            },
            _ => panic!("vftable cannot have non-function-pointer field"),
        })
        .collect()
}
