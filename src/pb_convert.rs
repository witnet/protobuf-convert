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

use darling::{FromDeriveInput, FromMeta};
use heck::SnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Path, Type, Variant};

use std::convert::TryFrom;

use super::{find_protobuf_convert_meta, DEFAULT_ONEOF_FIELD_NAME, PB_SNAKE_CASE_ATTRIBUTE};

#[derive(Debug, FromMeta)]
#[darling(default)]
struct ProtobufConvertStructAttrs {
    source: Option<Path>,
    serde_pb_convert: bool,
}

impl Default for ProtobufConvertStructAttrs {
    fn default() -> Self {
        Self {
            source: None,
            serde_pb_convert: false,
        }
    }
}

impl TryFrom<&[Attribute]> for ProtobufConvertStructAttrs {
    type Error = darling::Error;

    fn try_from(args: &[Attribute]) -> Result<Self, Self::Error> {
        find_protobuf_convert_meta(args)
            .map(|meta| Self::from_nested_meta(&meta))
            .unwrap_or_else(|| Ok(Self::default()))
    }
}

#[derive(Debug, FromMeta)]
#[darling(default)]
struct ProtobufConvertEnumAttrs {
    source: Option<Path>,
    serde_pb_convert: bool,
    impl_from_trait: bool,
    rename: Rename,
    oneof_field: Ident,
}

impl Default for ProtobufConvertEnumAttrs {
    fn default() -> Self {
        Self {
            source: None,
            oneof_field: syn::parse_str(DEFAULT_ONEOF_FIELD_NAME).unwrap(),
            serde_pb_convert: false,
            impl_from_trait: false,
            rename: Default::default(),
        }
    }
}

impl TryFrom<&[Attribute]> for ProtobufConvertEnumAttrs {
    type Error = darling::Error;

    fn try_from(args: &[Attribute]) -> Result<Self, Self::Error> {
        find_protobuf_convert_meta(args)
            .map(|meta| Self::from_nested_meta(&meta))
            .unwrap_or_else(|| Ok(Self::default()))
    }
}

#[derive(Debug)]
struct ProtobufConvertStruct {
    name: Ident,
    fields: Vec<(Ident, ProtobufConvertFieldAttrs)>,
    attrs: ProtobufConvertStructAttrs,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
struct ProtobufConvertFieldAttrs {
    skip: bool,
    with: Option<Path>,
}

impl TryFrom<&[Attribute]> for ProtobufConvertFieldAttrs {
    type Error = darling::Error;

    fn try_from(args: &[Attribute]) -> Result<Self, Self::Error> {
        find_protobuf_convert_meta(args)
            .map(|meta| Self::from_nested_meta(&meta))
            .unwrap_or_else(|| Ok(Self::default()))
    }
}

fn get_field_names(
    data: &DataStruct,
) -> Result<Vec<(Ident, ProtobufConvertFieldAttrs)>, darling::Error> {
    data.fields
        .iter()
        .map(|f| {
            let attrs = ProtobufConvertFieldAttrs::try_from(f.attrs.as_ref())?;
            let ident = f.ident.clone().ok_or_else(|| {
                darling::Error::unsupported_shape("Struct fields must have an identifier.")
            })?;

            Ok((ident, attrs))
        })
        .collect()
}

impl ProtobufConvertStruct {
    fn from_derive_input(
        name: Ident,
        data: &DataStruct,
        attrs: &[Attribute],
    ) -> Result<Self, darling::Error> {
        let attrs = ProtobufConvertStructAttrs::try_from(attrs)?;
        let fields = get_field_names(data)?;

        Ok(Self {
            name,
            attrs,
            fields,
        })
    }
}

impl ProtobufConvertFieldAttrs {
    fn impl_field_setter(&self, ident: &Ident) -> impl ToTokens {
        let pb_getter = Ident::new(&format!("get_{}", ident), Span::call_site());

        let setter = match (self.skip, &self.with) {
            // Usual setter.
            (false, None) => quote! { ProtobufConvert::from_pb(pb.#pb_getter().to_owned())? },
            // Setter with the overridden Protobuf conversion.
            (false, Some(with)) => quote! { #with::from_pb(pb.#pb_getter().to_owned())? },
            // Default setter for the skipped fields.
            (true, _) => quote! { Default::default() },
        };

        quote! { #ident: #setter, }
    }

    fn impl_field_getter(&self, ident: &Ident) -> impl ToTokens {
        let pb_setter = Ident::new(&format!("set_{}", ident), Span::call_site());

        match (self.skip, &self.with) {
            // Usual getter.
            (false, None) => quote! {
                msg.#pb_setter(ProtobufConvert::to_pb(&self.#ident).into());
            },
            // Getter with the overridden Protobuf conversion.
            (false, Some(with)) => quote! {
                msg.#pb_setter(#with::to_pb(&self.#ident).into());
            },
            // Skipped getter does nothing.
            (true, _) => quote! {},
        }
    }
}

impl ToTokens for ProtobufConvertStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let pb_name = &self.attrs.source;

        let from_pb_impl = {
            let fields = self
                .fields
                .iter()
                .map(|(ident, attrs)| attrs.impl_field_setter(ident));

            quote! {
                let inner = Self {
                    #(#fields)*
                };
                Ok(inner)
            }
        };

        let to_pb_impl = {
            let fields = self
                .fields
                .iter()
                .map(|(ident, attrs)| attrs.impl_field_getter(ident));

            quote! {
                let mut msg = Self::ProtoStruct::default();
                #(#fields)*
                msg
            }
        };

        let expanded = quote! {
            impl ProtobufConvert for #name {
                type ProtoStruct = #pb_name;

                fn from_pb(pb: Self::ProtoStruct) -> std::result::Result<Self, exonum_proto::_reexports::failure::Error> {
                    #from_pb_impl
                }

                fn to_pb(&self) -> Self::ProtoStruct {
                    #to_pb_impl
                }
            }
        };
        tokens.extend(expanded);
    }
}

#[derive(Debug)]
struct ParsedVariant {
    name: Ident,
    field_name: Path,
}

impl TryFrom<&Variant> for ParsedVariant {
    type Error = darling::Error;

    fn try_from(value: &Variant) -> Result<Self, Self::Error> {
        let name = value.ident.clone();
        let field_name = match &value.fields {
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(darling::Error::unsupported_shape(
                        "Too many fields in the enum variant",
                    ));
                }

                match &fields.unnamed.first().unwrap().ty {
                    Type::Path(type_path) => Ok(type_path.path.clone()),
                    _ => Err(darling::Error::unsupported_shape(
                        "Only variants in form Foo(Bar) are supported.",
                    )),
                }
            }
            _ => Err(darling::Error::unsupported_shape(
                "Only variants in form Foo(Bar) are supported.",
            )),
        }?;

        Ok(Self { name, field_name })
    }
}

#[derive(Debug)]
struct ProtobufConvertEnum {
    name: Ident,
    variants: Vec<ParsedVariant>,
    attrs: ProtobufConvertEnumAttrs,
}

#[derive(Debug, Default, FromMeta)]
#[darling(default)]
pub struct Rename {
    case: Option<String>,
}

impl ProtobufConvertEnum {
    fn from_derive_input(
        name: Ident,
        data: &DataEnum,
        attrs: &[Attribute],
    ) -> Result<Self, darling::Error> {
        let attrs = ProtobufConvertEnumAttrs::try_from(attrs)?;
        let variants = data
            .variants
            .iter()
            .map(ParsedVariant::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            name,
            attrs,
            variants,
        })
    }

    fn impl_protobuf_convert(&self) -> impl ToTokens {
        let pb_oneof_enum = {
            let mut pb = self.attrs.source.clone().unwrap();
            let oneof = pb.segments.pop().unwrap().value().ident.clone();
            let oneof_enum = Ident::new(
                &format!("{}_oneof_{}", oneof, &self.attrs.oneof_field),
                Span::call_site(),
            );
            quote! { #pb #oneof_enum }
        };
        let name = &self.name;
        let pb_name = &self.attrs.source;
        let oneof = &self.attrs.oneof_field;

        let from_pb_impl = {
            let match_arms = self.variants.iter().map(|variant| {
                let variant_name = self.get_variant_name(variant);
                let pb_variant = Ident::new(variant_name.as_ref(), Span::call_site());
                let variant_name = &variant.name;
                let field_name = &variant.field_name;

                quote! {
                    Some(#pb_oneof_enum::#pb_variant(pb)) => {
                        #field_name::from_pb(pb).map(#name::#variant_name)
                    }
                }
            });

            quote! {
                match pb.#oneof {
                    #( #match_arms )*
                    None => Err(exonum_proto::_reexports::failure::format_err!("Failed to decode #name from protobuf"))
                }
            }
        };
        let to_pb_impl = {
            let match_arms = self.variants.iter().map(|variant| {
                let pb_variant = self.get_variant_name(variant);
                let variant_name = &variant.name;

                let setter = Ident::new(&format!("set_{}", pb_variant), Span::call_site());
                quote! {
                    #name::#variant_name(msg) => inner.#setter(msg.to_pb()),
                }
            });

            quote! {
                let mut inner = Self::ProtoStruct::new();
                match self {
                    #( #match_arms )*
                }
                inner
            }
        };

        quote! {
            impl ProtobufConvert for #name {
                type ProtoStruct = #pb_name;

                fn from_pb(mut pb: Self::ProtoStruct) -> std::result::Result<Self, exonum_proto::_reexports::failure::Error> {
                    #from_pb_impl
                }

                fn to_pb(&self) -> Self::ProtoStruct {
                    #to_pb_impl
                }
            }
        }
    }

    fn impl_enum_conversions(&self) -> impl ToTokens {
        let name = &self.name;

        if self.attrs.impl_from_trait {
            let conversions = self.variants.iter().map(|variant| {
                let variant_name = &variant.name;
                let field_name = &variant.field_name;
                let variant_err = format!("Expected variant {}, but got {{:?}}", variant_name);

                quote! {
                    impl From<#field_name> for #name {
                       fn from(variant: #field_name) -> Self {
                           #name::#variant_name(variant)
                       }
                    }

                    impl std::convert::TryFrom<#name> for #field_name {
                        type Error = exonum_proto::_reexports::failure::Error;

                        fn try_from(msg: #name) -> Result<Self, Self::Error> {
                            if let #name::#variant_name(inner) = msg {
                                Ok(inner)
                            } else {
                                Err(exonum_proto::_reexports::failure::format_err!(
                                    #variant_err, msg
                                ))
                            }
                        }
                    }
                }
            });

            quote! {
                #( #conversions )*
            }
        } else {
            quote! {}
        }
    }

    fn get_variant_name(&self, variant: &ParsedVariant) -> String {
        if let Some(case) = self.attrs.rename.case.as_ref() {
            if case == PB_SNAKE_CASE_ATTRIBUTE {
                return variant.name.to_string().to_snake_case();
            } else {
                panic!("Undefined case type")
            }
        }

        variant.name.to_string()
    }
}

impl ToTokens for ProtobufConvertEnum {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let pb_convert = self.impl_protobuf_convert();
        let conversions = self.impl_enum_conversions();

        let expanded = quote! {
            #pb_convert
            #conversions
        };
        tokens.extend(expanded)
    }
}

#[derive(Debug)]
enum ProtobufConvert {
    Enum(ProtobufConvertEnum),
    Struct(ProtobufConvertStruct),
}

impl FromDeriveInput for ProtobufConvert {
    fn from_derive_input(input: &DeriveInput) -> Result<Self, darling::Error> {
        match &input.data {
            Data::Struct(data) => Ok(ProtobufConvert::Struct(
                ProtobufConvertStruct::from_derive_input(
                    input.ident.clone(),
                    data,
                    input.attrs.as_ref(),
                )?,
            )),
            Data::Enum(data) => Ok(ProtobufConvert::Enum(
                ProtobufConvertEnum::from_derive_input(
                    input.ident.clone(),
                    data,
                    input.attrs.as_ref(),
                )?,
            )),
            _ => Err(darling::Error::unsupported_shape(
                "Only for enums and structs.",
            )),
        }
    }
}

impl ProtobufConvert {
    fn name(&self) -> &Ident {
        match self {
            ProtobufConvert::Enum(inner) => &inner.name,
            ProtobufConvert::Struct(inner) => &inner.name,
        }
    }

    fn serde_needed(&self) -> bool {
        match self {
            ProtobufConvert::Enum(inner) => inner.attrs.serde_pb_convert,
            ProtobufConvert::Struct(inner) => inner.attrs.serde_pb_convert,
        }
    }

    fn implement_serde_protobuf_convert(&self) -> impl ToTokens {
        let name = self.name();
        quote! {
            impl serde::Serialize for #name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    self.to_pb().serialize(serializer)
                }
            }

            impl<'de> serde::Deserialize<'de> for #name {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let pb = <#name as ProtobufConvert>::ProtoStruct::deserialize(deserializer)?;
                    ProtobufConvert::from_pb(pb).map_err(serde::de::Error::custom)
                }
            }
        }
    }

    fn implement_protobuf_convert(&self) -> impl ToTokens {
        match self {
            ProtobufConvert::Enum(data) => quote! { #data },
            ProtobufConvert::Struct(data) => quote! { #data },
        }
    }
}

impl ToTokens for ProtobufConvert {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mod_name = Ident::new(
            &format!("pb_convert_impl_{}", self.name()),
            Span::call_site(),
        );
        let protobuf_convert = self.implement_protobuf_convert();
        let serde_traits = if self.serde_needed() {
            let serde = self.implement_serde_protobuf_convert();
            quote! { #serde }
        } else {
            quote! {}
        };

        let expanded = quote! {
            mod #mod_name {
                use super::*;

                use protobuf::Message as _ProtobufMessage;

                #protobuf_convert
                #serde_traits
            }
        };
        tokens.extend(expanded)
    }
}

pub fn implement_protobuf_convert(input: TokenStream) -> TokenStream {
    let input = ProtobufConvert::from_derive_input(&syn::parse(input).unwrap())
        .unwrap_or_else(|e| panic!("ProtobufConvert: {}", e));
    let tokens = quote! {#input};
    tokens.into()
}
