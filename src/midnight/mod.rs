//! Midnight blockchain specific functionality
//!
//! This module contains code specific to Midnight's blockchain implementation:
//! - Block digest parsing (slot extraction)
//! - Substrate keystore loading
//! - Validator registration checking
//! - Validator set management and block author attribution

pub mod digest;
pub mod keystore;
pub mod known_validators;
pub mod registration;
pub mod scale;
pub mod timing;
pub mod validators;

pub use digest::extract_slot_from_digest;
pub use keystore::{KeyStatus, ValidatorKeys};
pub use known_validators::KnownValidators;
pub use registration::{get_key_status, RegistrationStatus};
pub use scale::decode_aura_authorities;
pub use timing::{ChainTiming, Network};
pub use validators::ValidatorSet;
