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

use protobuf_convert::ProtobufConvert;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use crate::proto::ProtobufConvert;

mod proto;

#[derive(Debug, Clone, ProtobufConvert, Eq, PartialEq, Serialize, Deserialize)]
#[protobuf_convert(source = "proto::SkipFieldsMessage")]
struct SkipFieldsMessage {
    id: u32,
    #[protobuf_convert(skip)]
    #[serde(skip)]
    name: String,
}

#[derive(Debug, Clone, ProtobufConvert, Eq, PartialEq)]
#[protobuf_convert(source = "proto::SimpleMessage")]
struct Message {
    id: u32,
    name: String,
}

#[derive(Debug, ProtobufConvert, Eq, PartialEq)]
#[protobuf_convert(
    source = "proto::EnumMessage",
    impl_from_trait,
    rename(case = "snake_case")
)]
enum EnumMessage {
    Simple(Message),
    Skip(SkipFieldsMessage),
}

#[derive(Debug, ProtobufConvert, Eq, PartialEq)]
#[protobuf_convert(
    source = "proto::EnumMessageWithSimilarFields",
    rename(case = "snake_case")
)]
enum EnumMessageWithSimilarFields {
    Simple(Message),
    Skip(Message),
}

#[derive(Debug, ProtobufConvert, Eq, PartialEq)]
#[protobuf_convert(source = "proto::EnumMessageWithUpperCaseField")]
enum EnumMessageWithUpperCaseField {
    Simple(Message),
}

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

#[derive(Debug, ProtobufConvert, Eq, PartialEq)]
#[protobuf_convert(
    source = "proto::EnumMessage",
    rename(case = "snake_case"),
    serde_pb_convert
)]
enum EnumMessageWithSerde {
    Simple(Message),
    Skip(SkipFieldsMessage),
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

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn to_pb(v: &Option<CustomId>) -> u32 {
        match v {
            Some(id) => *id as u32,
            None => 0,
        }
    }
}

#[test]
fn simple_message_roundtrip() {
    let message = Message {
        id: 1,
        name: "SimpleMessage".into(),
    };
    let pb_message = message.to_pb();
    let de_message = Message::from_pb(pb_message).unwrap();

    assert_eq!(message, de_message);
}

#[test]
fn skip_field_message() {
    let message = SkipFieldsMessage {
        id: 1,
        name: "SimpleMessage".into(),
    };
    let pb_message = message.to_pb();
    let de_message = SkipFieldsMessage::from_pb(pb_message).unwrap();

    assert_eq!(message.id, de_message.id);
    assert!(de_message.name.is_empty());
}

#[test]
fn custom_message_roundtrip() {
    let message = CustomMessage {
        id: None,
        name: "SimpleMessage".into(),
    };
    let pb_message = message.to_pb();
    let de_message = CustomMessage::from_pb(pb_message).unwrap();

    assert_eq!(message, de_message);

    // Check `from_pb` with the unknown enum discriminant.
    let message = Message {
        id: 12,
        name: "Weird message".into(),
    };
    let pb_message = message.to_pb();

    let e = CustomMessage::from_pb(pb_message).unwrap_err();
    assert_eq!(e.to_string(), "Unknown enum discriminant: 12")
}

#[test]
fn enum_message() {
    let message = SkipFieldsMessage {
        id: 1,
        name: "SimpleMessage".into(),
    };

    let enum_message = EnumMessage::Skip(message.clone());

    let pb_message = enum_message.to_pb();
    let de_message = EnumMessage::from_pb(pb_message).unwrap();

    match de_message {
        EnumMessage::Skip(msg) => assert_eq!(msg.id, message.id),
        _ => panic!("Deserialized message has wrong type"),
    }
}

#[test]
fn from_trait() {
    let message = Message {
        id: 1,
        name: "message".into(),
    };
    let converted = EnumMessage::from(message.clone());

    match converted {
        EnumMessage::Simple(msg) => assert_eq!(msg.id, message.id),
        _ => panic!("Converted message has wrong type"),
    };

    let skip = SkipFieldsMessage {
        id: 1,
        name: "skip".into(),
    };
    let converted = EnumMessage::from(skip);
    let err = Message::try_from(converted).unwrap_err();

    assert!(err
        .to_string()
        .contains("Expected variant Simple, but got Skip"));
}

#[test]
fn serde_serialize_message() {
    let message = SkipFieldsMessage {
        id: 1,
        name: "SimpleMessage".into(),
    };

    let enum_message = EnumMessageWithSerde::Skip(message.clone());
    let pb_message = enum_message.to_pb();
    let de_message = EnumMessageWithSerde::from_pb(pb_message).unwrap();

    match de_message {
        EnumMessageWithSerde::Skip(msg) => assert_eq!(msg.id, message.id),
        _ => panic!("Deserialized message has wrong type"),
    }
}
