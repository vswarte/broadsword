use syn::{parse_macro_input, DeriveInput, Data};

extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

mod util;
mod decode;

#[proc_macro_derive(Codec, attributes(codec_assert, codec_context, codec_context_bind, codec_condition))]
pub fn codec(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);

    let context_attr = util::select_attributes(parsed.attrs.as_slice(), "codec_context");
    let context_type: proc_macro2::TokenStream = context_attr.first()
        .expect("Did not find required codec context, use codec_context attribute")
        .parse_args()
        .expect("Could not parse args to codec_context");

    let decode_impl = match parsed.data {
        Data::Struct(s) => decode::build_decode_impl(s),
        _ => todo!("Implement data type"),
    };

    let ident = &parsed.ident;
    proc_macro::TokenStream::from(quote! {
        impl Codec<#ident> for #ident {
            type CodecContext = #context_type;

            #decode_impl
        }
    })
}
