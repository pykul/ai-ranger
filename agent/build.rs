//! Prost build script: generates Rust types from .proto files at compile time.
//!
//! This is the idiomatic Rust approach -- code generation happens via cargo build,
//! not via make proto. Python and Go generation is handled by proto/Makefile.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(
        &[
            "../proto/ranger/v1/events.proto",
            "../proto/ranger/v1/agent.proto",
        ],
        &["../proto"],
    )?;
    Ok(())
}
