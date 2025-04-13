# #[allow(dead_code)]
# fn syn_field(input: syn::Field) -> syn::Field {
#     input
# }
#
# #[allow(dead_code)]
# fn field_attributes(input: syn::Field) -> Vec<proc_micro::WithSpan<ParseAttribute>> {
#     proc_micro::parse_attrs(&proc_micro::AttrNamespace("macro_name"), &input.attrs)
#         .to_result()
#         .unwrap()
# }
#
# #[derive(strum::EnumDiscriminants, Debug, PartialEq)]
# #[strum_discriminants(
#     name(KnownAttribute),
#     derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
# )]
# #[allow(dead_code)]
# enum ParseAttribute {
#     #[allow(non_camel_case_types)]
#     #[allow(dead_code)]
#     rename(String),
#     #[allow(non_camel_case_types)]
#     #[allow(dead_code)]
#     ignore,
# }
#
# impl syn::parse::Parse for ParseAttribute {
#     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
#         let ident = input.parse::<syn::Ident>()?;
#         match proc_micro::known_attribute(&ident)? {
#             KnownAttribute::rename => {
#                 input.parse::<syn::Token![=]>()?;
#                 Ok(ParseAttribute::rename(
#                     input.parse::<syn::LitStr>()?.value(),
#                 ))
#             }
#             KnownAttribute::ignore => Ok(ParseAttribute::ignore),
#         }
#     }
# }
#
