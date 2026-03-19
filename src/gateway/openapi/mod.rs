//! Stable `OpenAPI` contract surface for the Wendao gateway.

mod document;
pub mod paths;

pub use document::{
    bundled_wendao_gateway_openapi_document, bundled_wendao_gateway_openapi_path,
    load_bundled_wendao_gateway_openapi_document,
};
