pub use anyhow::Result;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse as number")]
    InvalidNumber(#[from] std::num::ParseIntError),
    #[error("/sys/kernel/config/nvmet does not exist. Are the nvmet modules loaded?")]
    NoNvmetSysfs,
    #[error("NVMe Qualified Name is not ASCII-only: {0}")]
    NQNNotAscii(String),
    #[error("NVMe Qualified Name is shorter than 13 bytes: {0}")]
    NQNTooShort(String),
    #[error("NVMe Qualified Name is longer than 223 bytes: {0}")]
    NQNTooLong(String),
    #[error("NVMe Qualified Name does not start with 'nqn.': {0}")]
    NQNMissingNQN(String),
    #[error("NVMe Qualified Name in UUID-Format does not have valid UUID: {0}")]
    NQNUuidInvalid(String),
    #[error("NVMe Qualified Name has an invalid date: {0}")]
    NQNInvalidDate(String),
    #[error("NVMe Qualified Name should not use org.nvmexpress unless it is a UUID: {0}")]
    NQNInvalidDomain(String),
    #[error("NVMe Qualified Name has invalid reverse domain or identifier: {0}")]
    NQNInvalidIdentifier(String),
    #[error("Unsupported addr_trtype: {0}")]
    UnsupportedTrType(String),
    #[error("Failed to parse IP address")]
    InvalidIPAddr(#[from] std::net::AddrParseError),
    #[error("Invalid FibreChannel addr_traddr: expected format nn-0x1000000044001123:pn-0x2000000055001123 or nn-1000000044001123:pn-2000000055001123: {0}")]
    InvalidFCAddr(String),
    #[error("Invalid Fibre Channel WWNN: {0}")]
    InvalidFCWWNN(String),
    #[error("Invalid Fibre Channel WWPN: {0}")]
    InvalidFCWWPN(String),
    #[error("No port with ID {0}")]
    NoSuchPort(u16),
    #[error("No subsystem with NQN {0}")]
    NoSuchSubsystem(String),
    #[error("Subsystem with NQN {0} cannot be created - it already exists")]
    ExistingSubsystem(String),
    #[error("Cannot create Subsystem with discovery NQN nqn.2014-08.org.nvmexpress.discovery")]
    CantCreateDiscovery,
    #[error("Subsystem model is invalid: {0} (ASCII printable characters only and 1-40 bytes)")]
    InvalidModel(String),
    #[error("Subsystem serial is invalid: {0} (ASCII printable characters only and 1-20 bytes)")]
    InvalidSerial(String),
    #[error("No such Host NQN: {0}")]
    NoSuchHost(String),
    #[error("Invalid Device: {0}")]
    InvalidDevice(String),
    #[error("Invalid namespace ID {0} - must not be 0 or NVME_NSID_ALL (4294967295)")]
    InvalidNamespaceID(u32),
    #[error("No namespace {0} in Subsystem {1}")]
    NoSuchNamespace(u32, String),
    #[error("Namespace {0} in Subsystem {1} cannot be created - it already exists")]
    ExistingNamespace(u32, String),
    #[error("Invalid UUID")]
    InvalidUuid(#[from] uuid::Error),
    #[error("Requested update, but specified no changes")]
    UpdateNoChanges,
    #[error("Unsupported config version: {0}")]
    UnsupportedConfigVersion(u32),
}
