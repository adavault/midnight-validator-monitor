//! Midnight blockchain specific functionality
//!
//! This module contains code specific to Midnight's blockchain implementation:
//! - Block digest parsing (slot extraction)
//! - Substrate keystore loading
//! - Validator registration checking

pub mod digest;
pub mod keystore;
pub mod registration;

pub use digest::extract_slot_from_digest;
pub use keystore::{normalize_hex, KeyStatus, ValidatorKeys};
pub use registration::{check_key_loaded, check_registration, get_key_status, RegistrationStatus};
