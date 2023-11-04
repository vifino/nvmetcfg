#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse as number")]
    InvalidNumber(#[from] std::num::ParseIntError),
    #[error("/sys/kernel/config/nvmet does not exist. Are the nvmet modules loaded?")]
    NoNvmetSysfs,
    #[error("NQN is not ASCII-only: {0}")]
    NQNNotAscii(String),
    #[error("Unsupported addr_trtype: {0}")]
    UnsupportedTrType(String),
    #[error("Failed to parse IP address")]
    InvalidIPAddr(#[from] std::net::AddrParseError),
    #[error("Invalid FibreChannel addr_traddr: {0}")]
    InvalidFCAddr(String),
    #[error("No port with ID {0}")]
    NoSuchPort(u32),
    #[error("No subsystem with NQN {0}")]
    NoSuchSubsystem(String),
    #[error("Subsystem with NQN {0} cannot be created, it already exists.")]
    ExistingSubsystem(String),
    #[error("Subsystem model is invalid: {0} (ASCII printable characters only and 1-40 bytes)")]
    InvalidModel(String),
    #[error("Subsystem serial is invalid: {0} (ASCII printable characters only and 1-20 bytes)")]
    InvalidSerial(String),
    #[error("No such Host NQN: {0}")]
    NoSuchHost(String),
    #[error("Invalid Device: {0}")]
    InvalidDevice(String),
    #[error("No namespace {0} in Subsystem {1}")]
    NoSuchNamespace(u32, String),
    #[error("Invalid UUID")]
    InvalidUuid(#[from] uuid::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
