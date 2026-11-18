//! The messages Firefox's extension sends to NOMNOM, and how they travel:
//! Firefox <-(native messaging: u32 length + JSON)-> host <-(one JSON line over a local Named Pipe or Unix socket)-> app.

pub mod framing;
pub mod pipe;
pub mod protocol;

pub use protocol::{
    parse_request, Body, ErrorCode, Preset, PresetKind, ProtoError, Request, Response,
};
