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

use serde::{Deserialize, Serialize};

use crate::proto::ProtobufConvert;
use protobuf_convert::ProtobufConvert;
use std::convert::TryFrom;

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
    let converted = EnumMessage::from(skip.clone());
    let err = Message::try_from(converted).unwrap_err();

    assert!(err
        .to_string()
        .contains("Expected variant Simple, but got Skip"));
}
