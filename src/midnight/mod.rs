//! Midnight blockchain specific functionality
//!
//! This module contains code specific to Midnight's blockchain implementation:
//! - Block digest parsing (slot extraction)
//! - Substrate keystore loading
//! - Validator registration checking
//! - Validator set management and block author attribution
//! - Block prediction algorithm

pub mod digest;
pub mod keystore;
pub mod prediction;
pub mod registration;
pub mod scale;
pub mod validators;

pub use digest::extract_slot_from_digest;
pub use keystore::{KeyStatus, ValidatorKeys};
pub use prediction::{BlockPrediction, PredictionCalculator};
pub use registration::{get_key_status, RegistrationStatus};
pub use scale::decode_aura_authorities;
pub use validators::{Validator, ValidatorSet};
