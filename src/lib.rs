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

#![recursion_limit = "256"]

extern crate proc_macro;

mod pb_convert;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Lit, Meta, MetaList, MetaNameValue, NestedMeta, Path};

// Derive attribute names, used as
// `#[protobuf_convert( [ ATTRIBUTE_NAME = ATTRIBUTE_VALUE or ATTRIBUTE_NAME ],* )]`
const CRATE_PATH_ATTRIBUTE: &str = "crate";
const PB_CONVERT_ATTRIBUTE: &str = "pb";
const SERDE_PB_CONVERT_ATTRIBUTE: &str = "serde_pb_convert";

/// Derives `ProtobufConvert` trait.
///
/// Attributes:
///
/// * `#[protobuf_convert( pb = "path" )]`
/// Required. `path` is the name of the corresponding protobuf generated struct.
///
/// * `#[protobuf_convert( crate = "path" )]`
/// Optional. `path` is prefix of the crate (defaults to "crate").
///
/// * `#[protobuf_convert( serde_pb_convert )]`
/// Optional. Implements `serde::{Serialize, Deserialize}` using structs that were generated with
/// protobuf.
/// For example, it should be used if you want json representation of your struct
/// to be compatible with protobuf representation (including proper nesting of fields).
/// ```text
/// // For example, struct with `xxx::crypto::Hash` with this
/// // (de)serializer will be represented as
/// StructName {
///     "hash": {
///         data: [1, 2, ...]
///     },
///     // ...
/// }
///
/// // With default (de)serializer.
/// StructName {
///     "hash": "12af..." // HEX
///     // ...
/// }
/// ```
#[proc_macro_derive(ProtobufConvert, attributes(protobuf_convert))]
pub fn generate_protobuf_convert(input: TokenStream) -> TokenStream {
    pb_convert::implement_protobuf_convert(input)
}

/// Types should be imported with `crate::` prefix if inside crate
fn get_types_prefix(attrs: &[Attribute]) -> impl quote::ToTokens {
    let map_attrs = get_name_value_attributes(attrs);
    let crate_path = map_attrs.into_iter().find_map(|nv| match &nv {
        MetaNameValue {
            lit: Lit::Str(path),
            ident,
            ..
        } if ident == CRATE_PATH_ATTRIBUTE => Some(
            path.parse::<Path>()
                .expect("failed to parse crate path in the attribute"),
        ),
        _ => None,
    });

    if let Some(path) = crate_path {
        quote!(#path)
    } else {
        quote!(crate)
    }
}

/// Extract attributes in the form of `#[protobuf_convert(name = "value")]`
fn get_attributes(attrs: &[Attribute]) -> Vec<Meta> {
    let meta = attrs.iter().find_map(|attr| {
        attr.parse_meta()
            .ok()
            .filter(|m| m.name() == "protobuf_convert")
    });

    match meta {
        Some(Meta::List(MetaList { nested: list, .. })) => list
            .into_iter()
            .filter_map(|n| match n {
                NestedMeta::Meta(meta) => Some(meta),
                _ => None,
            })
            .collect(),
        Some(_) => panic!("`protobuf_convert` attribute should contain list of name value pairs"),
        None => vec![],
    }
}

fn get_name_value_attributes(attrs: &[Attribute]) -> Vec<MetaNameValue> {
    get_attributes(attrs)
        .into_iter()
        .filter_map(|meta| match meta {
            Meta::NameValue(name_value) => Some(name_value),
            _ => None,
        })
        .collect()
}

fn find_word_attribute(attrs: &[Attribute], ident_name: &str) -> bool {
    get_attributes(attrs).iter().any(|meta| match meta {
        Meta::Word(ident) if ident == ident_name => true,
        _ => false,
    })
}
