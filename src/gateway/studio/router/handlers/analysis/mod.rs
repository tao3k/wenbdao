//! Studio API endpoint handlers.

pub(crate) mod flight;
mod service;

pub(crate) use flight::{
    StudioCodeAstAnalysisFlightRouteProvider, StudioMarkdownAnalysisFlightRouteProvider,
};
