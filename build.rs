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

use std::{fs, io::prelude::*, path::Path};

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("Unable to get OUT_DIR");

    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(&out_dir)
        .input("tests/proto/message.proto")
        .include("tests/proto")
        .customize(
            protobuf_codegen::Customize::default()
                .generate_accessors(true)
                .gen_mod_rs(true),
        )
        .run_from_script();

    let mod_file_content = r#"pub use self::message::*; 

mod message;
"#;
    let mod_file_path = Path::new(&out_dir).join("mod.rs");

    let mut file = fs::File::create(&mod_file_path).expect("Unable to create mod.rs file");
    file.write_all(mod_file_content.to_string().as_ref())
        .expect("Unable to write mod.rs file");
}
