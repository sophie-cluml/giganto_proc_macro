
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DataStruct, DeriveInput, Fields, GenericArgument, PathArguments, Type};

extern crate proc_macro;

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(autogen_type))]
struct FromGraphQlAutogenStructAttrs {
    name: Type,
}

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(autogen_type))]
struct GraphQlAutogenFieldAttrs {
    #[deluxe(default = "".to_string())]
    pub name: String,
}

// fn extract_field_attrs(
//     ast: &mut DeriveInput,
// ) -> deluxe::Result<HashMap<String, GraphQlAutogenFieldAttrs>> {

//     if let syn::Data::Struct(s) = &mut ast.data {
//         for field in s.fields.iter_mut() {
//             let field_name =
//         }
//     }
// }

fn derive_from_graphql_client_autogen_2(
    item: proc_macro2::TokenStream,
) -> deluxe::Result<proc_macro2::TokenStream> {
    // start
    let mut ast: DeriveInput = syn::parse2(item)?;

    // extract idents
    let FromGraphQlAutogenStructAttrs { name } = deluxe::extract_attributes(&mut ast)?;
    let struct_ident = &ast.ident;

    // generate field conversions
    if let Data::Struct(DataStruct { fields, .. }) = &mut ast.data {
        let field_conversions = match fields {
            Fields::Named(named_fields) => named_fields.named.iter_mut().map(|field| {
                // eprintln!("FIELD : {field:?}");

                // eprintln!("FIELD.TYPE : {:?}", field.ty);
                let field_name = &field.ident.clone().unwrap();

                // let mut field_attrs = HashMap::<String, GraphQlAutogenFieldAttrs>::new();

                // let cust_name:GraphQlAutogenFieldAttrs  = deluxe::extract_attributes(field).unwrap();
                let from_field_name;

                let GraphQlAutogenFieldAttrs { name } = deluxe::extract_attributes(field).unwrap();
                if name.is_empty() {
                    from_field_name = field_name.clone();
                } else {
                    from_field_name = proc_macro2::Ident::from(name);
                }

                let field_type = &field.ty;
                let field_quoted = if is_map_cast_needed(field_type) {
                    quote! {
                        #field_name: node.#from_field_name.into_iter().map(|x| x as _).collect(),
                    }
                } else if is_target_of_force_cast(field_type) {
                    quote! {
                        #field_name: node.#from_field_name as _,
                    }
                } else {
                    quote! {
                        #field_name: node.#from_field_name,
                    }
                };

                field_quoted
            }),
            _ => {
                panic!("FromGraphQlAutogenType only support named fields");
            }
        };

        // generate code output
        let suppress_clippy_warning =
            quote! { #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] };
        Ok(quote! {
            impl From<#name> for #struct_ident {
                #suppress_clippy_warning
                fn from(node: #name) -> Self {
                    Self {
                        #( #field_conversions )*
                    }
                }
            }
        })
    } else {
        Ok(quote! {
            impl From<#name> for #struct_ident {
                fn from(node: #name) -> Self {
                    Self {}
                }
            }
        })
    }
}

fn is_map_cast_needed(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ret = segment.ident.to_string() == "Vec";
            if ret {
                if let PathArguments::AngleBracketed(vec_element_type_arg) = &segment.arguments {
                    for arg in vec_element_type_arg.args.iter() {
                        match arg {
                            GenericArgument::Type(vec_el_type) => {
                                return is_target_of_force_cast(&vec_el_type);
                            }
                            _ => continue,
                        }
                    }
                }
            } else {
                return ret;
            }
        }
    }
    false
}

static TARGET_OF_FORCE_CAST: [&str; 8] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "usize"];

fn is_target_of_force_cast(ty: &Type) -> bool {
    let type_token = ty.to_token_stream().to_string();
    TARGET_OF_FORCE_CAST.contains(&type_token.as_str())
}

/// Implements From trait for struct.
/// This macro is designed to be used "only" to convert graphql_client library's
/// autogenerated struct data into project's struct.
///
/// NOTE : this macro relies on lossy casting, `as _`, but this is not an issue.
/// It is because the integrity of data is solely upon the project's struct,
/// so data loss shall never happen.
/// Grphql_client library and graphql's interpretes all number types as i64
/// at its code autogen, hence it is necessary to force-cast.
///
/// LIMITATIONS : This macro can only support castings for limited cases.
/// Supported cases are Primitive types and Vec, and structs.
///
/// # Examples
/// ```no_run
///
/// // graphql_client_autogen.rs
/// use graphql_client::GraphQLQuery;
///
/// #[derive(GraphQLQuery)]
/// #[graphql(
///     schema_path = "src/graphql/client/schema/schema.graphql",
///     query_path = "src/graphql/client/schema/conn_raw_events.graphql",
///     response_derives = "Clone, Default, PartialEq"
/// )]
/// pub struct ConnRawEvents;
///
///
/// // main.rs
/// pub struct ConnRawEvents;
/// use giganto_proc_macro;
/// use graphql_client_autogen::conn_raw_events;
///
/// #[derive(SimpleObject, FromGraphQlClientAutogenType)]
/// #[autogen_type(name = conn_raw_events::ConnRawEventsConnRawEventsEdgesNode)]
/// struct ConnRawEvent {
///     timestamp: DateTime,
///     orig_port: u16,
///     proto: u8,
///     duration: i64,
///     service: String,
///     resp_pkts: u64,
///     ttl: Vec<i32>,
///     orig_filenames: Vec<String>,
///
///
/// }
///```
///
/// Above code expands to below.
/// ```
/// impl From<conn_raw_events::ConnRawEventsConnRawEventsEdgesNode> for ConnRawEvent {
///     #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
///     fn from(node: conn_raw_events::ConnRawEventsConnRawEventsEdgesNode) -> Self {
///         Self {
///             timestamp: node.timestamp,
///             orig_port: node.orig_port as _,
///             proto: node.proto as _,
///             duration: node.duration,
///             service: node.service as _,
///             resp_pkts: node.resp_pkts as _,
///             ttl: node.ttl.into_iter().map(|x| x as _).collect(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(FromGraphQlClientAutogenType, attributes(autogen_type))]
pub fn derive_from_graphql_client_autogen(input: TokenStream) -> TokenStream {
    derive_from_graphql_client_autogen_2(input.into())
        .unwrap()
        .into()
}
