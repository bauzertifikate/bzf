// SPDX-License-Identifier: Apache-2.0 OR MIT
//! BZF's rules as a library: canonical JSON and the hash a document is named by, the schemas, a project read from a
//! folder or a `.bzf` file and checked, the `.bzf` archive, and signatures over result records.
//!
//! The text is the specification (../../README.md, HASHES.md, SIGNATURES.md); this is one implementation of it,
//! written from that text.

pub mod archive;
pub mod canonical;
pub mod project;
pub mod schemas;
pub mod signature;
