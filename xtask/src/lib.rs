//! Repository harness: requirement traceability and PR-scope checks.
//!
//! This crate is a verification asset (GOV-04). It must not depend on any ABW product crate.

pub mod checks;
pub mod lex;
pub mod register;
pub mod results;
pub mod scope;
pub mod tags;
pub mod trace;
