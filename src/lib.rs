#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
pub mod ical;
pub mod template;
