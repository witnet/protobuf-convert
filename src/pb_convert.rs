// Copyright 2018 The Exonum Team, 2019 Witnet Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Lit, Path};

use super::{
    find_word_attribute, get_name_value_attributes,
    PB_CONVERT_ATTRIBUTE, SERDE_PB_CONVERT_ATTRIBUTE,
};

fn get_protobuf_struct_path(attrs: &[Attribute]) -> Path {
    let map_attrs = get_name_value_attributes(attrs);
    let struct_path = map_attrs.into_iter().find_map(|nv| {
        if nv.ident == PB_CONVERT_ATTRIBUTE {
            match nv.lit {
                Lit::Str(path) => Some(path.parse::<Path>().unwrap()),
                _ => None,
            }
        } else {
            None
        }
    });

    struct_path.unwrap_or_else(|| panic!("{} attribute is not set properly.", PB_CONVERT_ATTRIBUTE))
}

fn get_field_names(input: &DeriveInput) -> Option<Vec<Ident>> {
    let data = match &input.data {
        Data::Struct(x) => Some(x),
        Data::Enum(..) => None,
        _ => panic!("Protobuf convert can be derived for structs and enums only."),
    };
    data.map(|data| {
        data.fields
            .iter()
            .map(|f| f.ident.clone().unwrap())
            .collect()
    })
}

fn get_field_names_enum(input: &DeriveInput) -> Option<Vec<Ident>> {
    let data = match &input.data {
        Data::Struct(..) => None,
        Data::Enum(x) => Some(x),
        _ => panic!("Protobuf convert can be derived for structs and enums only."),
    };
    data.map(|data| data.variants.iter().map(|f| f.ident.clone()).collect())
}

fn implement_protobuf_convert_from_pb(field_names: &[Ident]) -> impl quote::ToTokens {
    let getters = field_names
        .iter()
        .map(|i| Ident::new(&format!("get_{}", i), Span::call_site()));
    let our_struct_names = field_names.to_vec();

    quote! {
        fn from_pb(pb: Self::ProtoStruct) -> std::result::Result<Self, _FailureError> {
          Ok(Self {
           #( #our_struct_names: ProtobufConvert::from_pb(pb.#getters().to_owned())?, )*
          })
        }
    }
}

fn implement_protobuf_convert_to_pb(field_names: &[Ident]) -> impl quote::ToTokens {
    let setters = field_names
        .iter()
        .map(|i| Ident::new(&format!("set_{}", i), Span::call_site()));
    let our_struct_names = field_names.to_vec();

    quote! {
        fn to_pb(&self) -> Self::ProtoStruct {
            let mut msg = Self::ProtoStruct::new();
            #( msg.#setters(ProtobufConvert::to_pb(&self.#our_struct_names).into()); )*
            msg
        }
    }
}

fn implement_protobuf_convert_trait(
    name: &Ident,
    pb_name: &Path,
    field_names: &[Ident],
) -> impl quote::ToTokens {
    let to_pb_fn = implement_protobuf_convert_to_pb(field_names);
    let from_pb_fn = implement_protobuf_convert_from_pb(field_names);

    quote! {
        impl ProtobufConvert for #name {
            type ProtoStruct = #pb_name;

            #to_pb_fn
            #from_pb_fn
        }
    }
}

fn implement_protobuf_convert_from_pb_enum(
    name: &Ident,
    pb_name: &Path,
    field_names: &[Ident],
) -> impl quote::ToTokens {
    let our_struct_names = field_names.to_vec();
    let our_struct_names1 = our_struct_names.clone();
    let name1 = name;
    let name = std::iter::repeat(name);
    let mut pb_name_kind = pb_name.clone();
    let pb_name_ident = Ident::new(
        &format!(
            "{}_oneof_kind",
            pb_name
                .segments
                .last()
                .unwrap()
                .into_tuple()
                .0
                .ident
                .to_string()
        ),
        Span::call_site(),
    );
    pb_name_kind
        .segments
        .last_mut()
        .unwrap()
        .into_tuple()
        .0
        .ident = pb_name_ident;
    let pb_name_kind = std::iter::repeat(pb_name_kind);

    quote! {
        fn from_pb(pb: Self::ProtoStruct) -> std::result::Result<Self, _FailureError> {
            Ok(match pb.kind {
            #(
                Some(#pb_name_kind::#our_struct_names(x)) => {
                    #name::#our_struct_names1(ProtobufConvert::from_pb(x)?)
                }
            )*
                None => return Err(failure::err_msg(format!("{}: Invalid Enum Variant", stringify!(#name1)))),
            })
        }
    }
}

fn implement_protobuf_convert_to_pb_enum(
    name: &Ident,
    _pb_name: &Path,
    field_names: &[Ident],
) -> impl quote::ToTokens {
    let setters = field_names
        .iter()
        .map(|i| Ident::new(&format!("set_{}", i), Span::call_site()));
    let our_struct_names = field_names.to_vec();
    let name = std::iter::repeat(name);

    quote! {
        fn to_pb(&self) -> Self::ProtoStruct {
            let mut msg = Self::ProtoStruct::new();
            match &self {
                #(
                #name::#our_struct_names(x) => {
                    msg.#setters(ProtobufConvert::to_pb(x).into());
                }
                )*
            }
            msg
        }
    }
}

fn implement_protobuf_convert_trait_enum(
    name: &Ident,
    pb_name: &Path,
    field_names: &[Ident],
) -> impl quote::ToTokens {
    let to_pb_fn = implement_protobuf_convert_to_pb_enum(name, pb_name, field_names);
    let from_pb_fn = implement_protobuf_convert_from_pb_enum(name, pb_name, field_names);

    quote! {
        impl ProtobufConvert for #name {
            type ProtoStruct = #pb_name;

            #to_pb_fn
            #from_pb_fn
        }
    }
}

fn implement_serde_protobuf_convert(name: &Ident) -> proc_macro2::TokenStream {
    quote! {
        extern crate serde as _serde;

        impl _serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: _serde::Serializer,
            {
                self.to_pb().serialize(serializer)
            }
        }

        impl<'de> _serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: _serde::Deserializer<'de>,
            {
                let pb = <#name as ProtobufConvert>::ProtoStruct::deserialize(deserializer)?;
                ProtobufConvert::from_pb(pb).map_err(_serde::de::Error::custom)
            }
        }
    }
}

pub fn implement_protobuf_convert(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    let name = input.ident.clone();
    let proto_struct_name = get_protobuf_struct_path(&input.attrs);

    let mod_name = Ident::new(&format!("pb_convert_impl_{}", name), Span::call_site());

    if let Some(field_names) = get_field_names(&input) {
        // for structs
        let protobuf_convert =
            implement_protobuf_convert_trait(&name, &proto_struct_name, &field_names);

        let serde_traits = {
            let serde_needed = find_word_attribute(&input.attrs, SERDE_PB_CONVERT_ATTRIBUTE);
            if serde_needed {
                implement_serde_protobuf_convert(&name)
            } else {
                quote!()
            }
        };

        let expanded = quote! {
            mod #mod_name {
                extern crate protobuf as _protobuf_crate;
                extern crate failure as _failure;

                use super::*;

                use self::_protobuf_crate::Message as _ProtobufMessage;
                use self::_failure::{bail, Error as _FailureError};

                #protobuf_convert
                #serde_traits
            }
        };

        expanded.into()
    } else if let Some(field_names) = get_field_names_enum(&input) {
        // for enums
        let protobuf_convert =
            implement_protobuf_convert_trait_enum(&name, &proto_struct_name, &field_names);

        let serde_traits = {
            let serde_needed = find_word_attribute(&input.attrs, SERDE_PB_CONVERT_ATTRIBUTE);
            if serde_needed {
                implement_serde_protobuf_convert(&name)
            } else {
                quote!()
            }
        };

        let expanded = quote! {
            mod #mod_name {
                extern crate protobuf as _protobuf_crate;
                extern crate failure as _failure;

                use super::*;

                use self::_protobuf_crate::Message as _ProtobufMessage;
                use self::_failure::{bail, Error as _FailureError};

                #protobuf_convert
                #serde_traits
            }
        };

        expanded.into()
    } else {
        quote!().into()
    }
}
