//! Web of Things application server
//!
//! Provides all the building blocks to serve [Web Of Things](https://www.w3.org/WoT/) Things.

pub mod advertise;
#[doc(hidden)]
pub mod hlist;
pub mod servient;

pub use servient::Servient;
