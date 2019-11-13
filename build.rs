extern crate protoc_rust;

use protoc_rust::Customize;

fn main() {
    protoc_rust::run(protoc_rust::Args {
        out_dir: "tests/proto",
        input: &["tests/proto/message.proto"],
        includes: &["tests/proto"],
        customize: Customize {
            ..Default::default()
        },
    })
    .expect("Couldn't compile proto sources");
}
