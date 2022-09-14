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

pub use message::*;

use anyhow::Error;

pub mod message {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

pub trait ProtobufConvert: Sized {
    /// Type of the protobuf clone of Self
    type ProtoStruct;

    /// Struct -> ProtoStruct
    fn to_pb(&self) -> Self::ProtoStruct;

    /// ProtoStruct -> Struct
    fn from_pb(pb: Self::ProtoStruct) -> Result<Self, Error>;
}

impl ProtobufConvert for String {
    type ProtoStruct = Self;

    fn to_pb(&self) -> Self::ProtoStruct {
        self.clone()
    }

    fn from_pb(pb: Self::ProtoStruct) -> Result<Self, Error> {
        Ok(pb)
    }
}

impl ProtobufConvert for u32 {
    type ProtoStruct = Self;

    fn to_pb(&self) -> Self::ProtoStruct {
        *self
    }

    fn from_pb(pb: Self::ProtoStruct) -> Result<Self, Error> {
        Ok(pb)
    }
}
