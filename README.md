# protobuf-convert

Macros for convenient serialization of Rust data structures into/from Protocol Buffers.

## Introduction

This is a fork of [exonum-derive](https://crates.io/crates/exonum-derive) with
some changes to allow easier integration with other projects, and some new
features.

## Usage

First, add the dependency in `Cargo.toml`:

```toml
protobuf-convert = "0.3.0"
```

Then, define a `ProtobufConvert` trait:

```rust
trait ProtobufConvert {
    /// Type of the protobuf clone of Self
    type ProtoStruct;

    /// Struct -> ProtoStruct
    fn to_pb(&self) -> Self::ProtoStruct;

    /// ProtoStruct -> Struct
    fn from_pb(pb: Self::ProtoStruct) -> Result<Self, Error>;
}
```

And to use it, import the trait and the macro:

For example, given the following protobuf:

```protobuf
message Ping {
    fixed64 nonce = 1;
}
```

rust-protobuf will generate the following struct:

```rust
#[derive(PartialEq,Clone,Default)]
#[cfg_attr(feature = "with-serde", derive(Serialize, Deserialize))]
pub struct Ping {
    // message fields
    pub nonce: u64,
    // special fields
    #[cfg_attr(feature = "with-serde", serde(skip))]
    pub unknown_fields: ::protobuf::UnknownFields,
    #[cfg_attr(feature = "with-serde", serde(skip))]
    pub cached_size: ::protobuf::CachedSize,
}
```

We may want to convert that struct into a more idiomatic one, and derive more traits.
This is the necessary code:

```rust
// Import trait
use crate::proto::ProtobufConvert;
// Import macro
use protobuf_convert::ProtobufConvert;
// Import module autogenerated by protocol buffers
use crate::proto::schema;

#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Ping")]
struct Ping {
    nonce: u64,
}
```

Note that the `ProtobufConvert` trait must be implemented for all the fields,
see an example implementation for `u64`:

```rust
impl ProtobufConvert for u64 {
    type ProtoStruct = u64;

    fn to_pb(&self) -> Self::ProtoStruct {
        *self
    }

    fn from_pb(pb: Self::ProtoStruct) -> Result<Self, Error> {
        Ok(pb)
    }
}
```

Now, converting between `Ping` and `schema::Ping` can be done effortlessly.

### `Enum` support

A more complex example, featuring enums:

```protobuf
message Ping {
    fixed64 nonce = 1;
}
message Pong {
    fixed64 nonce = 1;
}
message Message {
    oneof kind {
        Ping Ping = 1;
        Pong Pong = 2;
    }
}
```

```rust
#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Ping")]
struct Ping {
    nonce: u64,
}
#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Pong")]
struct Pong {
    nonce: u64,
}
#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Message")]
enum Message {
    Ping(Ping),
    Pong(Pong),
}
```

And it just works!

You can also generate `From` and `TryFrom` traits for enum variants. Note that this will not work if enum has variants
with the same field types. To use this feature add `impl_from_trait` attribute.

```rust
#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Message"), impl_from_trait]
enum Message {
    Ping(Ping),
    Pong(Pong),
}
```

`From<Ping>`, `From<Pong>` and also `TryFrom<..>` traits will be generated.

Another attribute that can be used with enum is `rename`. It instructs macro to generate methods with case
specified in attribute param.

```rust
#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Message"), rename(case = "snake_case")]
enum Message {
    Ping(Ping),
    Pong(Pong),
}
```

Currently, only snake case is supported.

### Skipping fields

This macro also supports skipping fields in `struct`s so they are ignored when serializing, i.e they will not be mapped to any field in the schema:

```rust
#[derive(ProtobufConvert)]
#[protobuf_convert(source = "schema::Ping")]
struct Ping {
    pub nonce: u64,
    #[protobuf_convert(skip)]
    my_private_field: u64
}
```

Note that you can only skip fields whose type implements the `Default` trait.

### Overriding conversion rules

This macro also supports serde-like attribute `with` for modules with the custom implementation of `from_pb` and `to_pb` conversions.

`protobuf-convert` will use functions `$module::from_pb` and `$module::to_pb` instead of `ProtobufConvert` trait for the specified field.

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CustomId {
    First = 5,
    Second = 15,
    Third = 35,
}

#[derive(Debug, Clone, ProtobufConvert, Eq, PartialEq)]
#[protobuf_convert(source = "proto::SimpleMessage")]
struct CustomMessage {
    #[protobuf_convert(with = "custom_id_pb_convert")]
    id: Option<CustomId>,
    name: String,
}

mod custom_id_pb_convert {
    use super::*;

    pub(super) fn from_pb(pb: u32) -> Result<Option<CustomId>, anyhow::Error> {
        match pb {
            0 => Ok(None),
            5 => Ok(Some(CustomId::First)),
            15 => Ok(Some(CustomId::Second)),
            35 => Ok(Some(CustomId::Third)),
            other => Err(anyhow::anyhow!("Unknown enum discriminant: {}", other)),
        }
    }

    pub(super) fn to_pb(v: &Option<CustomId>) -> u32 {
        match v {
            Some(id) => *id as u32,
            None => 0,
        }
    }
}
```

## See also

* [rust-protobuf](https://github.com/stepancheg/rust-protobuf)
