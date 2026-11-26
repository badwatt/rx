//! Cross-cutting utilities that are not part of the business domain.

pub(crate) mod ansi;
pub(crate) mod osc;
pub(crate) mod progress;

#[cfg(test)]
pub(crate) mod test_helpers;
