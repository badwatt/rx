//! Use-case slice: the NixOS update workflow.
//!
//! The hexagonal layers live inside the feature: `application` (use cases),
//! `ports` (the interfaces it needs) and `adapters` (driven implementations).
//! It shares only the domain kernel and generic infrastructure.

pub(crate) mod adapters;
pub(crate) mod application;
pub(crate) mod ports;
