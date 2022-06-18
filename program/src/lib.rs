pub mod swap;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;


// todo: set mainnet / dev value
solana_program::declare_id!("FGkaF25KUXq8RXCr8wPTFwbUnnQA6bFzwNj1mh9k6prK");