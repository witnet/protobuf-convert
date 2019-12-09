// Copyright 2019 The Exonum Team, 2019 Witnet Foundation
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
use syn::{Attribute, NestedMeta};

const PB_CONVERT_ATTRIBUTE: &str = "protobuf_convert";
const PB_SNAKE_CASE_ATTRIBUTE: &str = "snake_case";
const DEFAULT_ONEOF_FIELD_NAME: &str = "kind";

/// ProtobufConvert derive macro.
///
/// Attributes:
///
/// ## Required
///
/// * `#[protobuf_convert(source = "path")]`
///
/// ```ignore
/// #[derive(Clone, Debug, ProtobufConvert)]
/// #[protobuf_convert(source = "proto::Message")]
/// pub struct Message {
///     /// Message author id.
///     pub author: u32,
///     /// Message text.
///     pub text: String,
/// }
///
/// let msg = Message::new();
/// let serialized_msg = msg.to_pb();
///
/// let deserialized_msg = ProtobufConvert::from_pb(serialized_msg).unwrap();
/// assert_eq!(msg, deserialized_msg);
/// ```
///
/// Corresponding proto file:
/// ```proto
/// message Message {
///     // Message author id..
///     uint32 author = 1;
///     // Message text.
///     string text = 2;
/// }
/// ```
///
/// This macro can also be applied to enums. In proto files enums are represented
/// by `oneof` field. You can specify `oneof` field name, default is "kind".
/// Corresponding proto file must contain only this oneof field. Possible enum
/// variants are zero-field and one-field variants.
/// Another enum attribute is `impl_from_trait`. If you specify it then `From` and `TryFrom`
/// traits for enum variants will be generated. Note that this will not work if enum has
/// variants with the same field types.
/// ```ignore
/// #[derive(Debug, Clone, ProtobufConvert)]
/// #[protobuf_convert(source = "proto::Message", oneof_field = "message")]
/// pub enum Message {
///     /// Plain message.
///     Plain(String),
///     /// Encoded message.
///     Encoded(String),
/// }
/// ```
///
/// Corresponding proto file:
/// ```proto
/// message Message {
///     oneof message {
///         // Plain message.
///         string plain = 1;
///         // Encoded message.
///         string encoded = 2;
///     }
/// }
/// ```
///
/// Path is the name of the corresponding protobuf generated struct.
///
/// * `#[protobuf_convert(source = "path", serde_pb_convert)]`
///
/// Implement `serde::{Serialize, Deserialize}` using structs that were generated with
/// protobuf.
/// For example, it should be used if you want json representation of your struct
/// to be compatible with protobuf representation (including proper nesting of fields).
/// For example, struct with `crypto::Hash` with this
/// (de)serializer will be represented as
/// ```text
/// StructName {
///     "hash": {
///         "data": [1, 2, ...]
///     },
///     // ...
/// }
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

pub(crate) fn find_protobuf_convert_meta(args: &[Attribute]) -> Option<NestedMeta> {
    args.as_ref()
        .iter()
        .filter_map(|a| a.parse_meta().ok())
        .find(|m| m.path().is_ident(PB_CONVERT_ATTRIBUTE))
        .map(NestedMeta::from)
}
