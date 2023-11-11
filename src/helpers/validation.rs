use crate::errors::{Error, Result};
use uuid::Uuid;

#[must_use]
pub fn is_ascii_only(data: &str) -> bool {
    for c in data.chars() {
        if !c.is_ascii() && !c.is_ascii_control() {
            return false;
        }
    }
    true
}

pub fn assert_valid_nqn(nqn: &str) -> Result<()> {
    if !is_ascii_only(nqn) {
        Err(Error::NQNNotAscii(nqn.to_string()).into())
    } else if nqn.len() > 223 {
        Err(Error::NQNTooLong(nqn.to_string()).into())
    } else {
        Ok(())
    }
}

pub fn assert_valid_subsys_name(nqn: &str) -> Result<()> {
    assert_valid_nqn(nqn)?;
    if nqn == "nqn.2014-08.org.nvmexpress.discovery" {
        Err(Error::CantCreateDiscovery.into())
    } else {
        Ok(())
    }
}

pub fn assert_compliant_nqn(nqn: &str) -> Result<()> {
    assert_valid_nqn(nqn)?;
    if !nqn.starts_with("nqn.") {
        Err(Error::NQNMissingNQN(nqn.to_string()).into())
    } else if let Some(uuid) = nqn.strip_prefix("nqn.2014-08.org.nvmexpress:uuid:") {
        // NQN is a UUID. So we should ensure it's valid.
        if Uuid::try_parse(uuid).is_err() {
            Err(Error::NQNUuidInvalid(uuid.to_string()).into())
        } else {
            Ok(())
        }
    } else {
        // TODO: check if nqn has nqn.yyyy-mm, some reverse domain and a colon.
        // we can't make many other assumptions.
        Ok(())
    }
}

pub fn assert_valid_model(model: &str) -> Result<()> {
    if !is_ascii_only(model) && (model.len() <= 40) {
        Err(Error::InvalidModel(model.to_string()).into())
    } else {
        Ok(())
    }
}
pub fn assert_valid_serial(model: &str) -> Result<()> {
    if !is_ascii_only(model) && !model.is_empty() && (model.len() <= 40) {
        Err(Error::InvalidModel(model.to_string()).into())
    } else {
        Ok(())
    }
}

pub fn assert_valid_nsid(nsid: u32) -> Result<()> {
    if nsid == 0 || nsid == 0xffff_ffff {
        Err(Error::InvalidNamespaceID(nsid).into())
    } else {
        Ok(())
    }
}
