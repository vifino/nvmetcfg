#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse as number")]
    InvalidNumber(#[from] std::num::ParseIntError),
    #[error("Unsupported addr_trtype: {0}")]
    UnsupportedTrType(String),
    #[error("Failed to parse IP address")]
    InvalidIPAddr(#[from] std::net::AddrParseError),
    #[error("Invalid FibreChannel addr_traddr: {0}")]
    InvalidFCAddr(String),
}

pub type Result<T> = std::result::Result<T, Error>;
