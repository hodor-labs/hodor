pub mod swap;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;


// todo: set mainnet / dev value
solana_program::declare_id!("9UxBkKXXfr5z1eMYSXs3DBsjDMKP7D1CaQ9sNhGxTRM9");