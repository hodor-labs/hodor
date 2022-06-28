pub mod swap;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("hodor77UwFjwKS9cwfkoT3HHi1rofTLfYaFBeGDBngo");