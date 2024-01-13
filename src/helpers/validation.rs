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
    if !is_ascii_only(model) || model.is_empty() || (model.len() > 40) {
        Err(Error::InvalidModel(model.to_string()).into())
    } else {
        Ok(())
    }
}
pub fn assert_valid_serial(serial: &str) -> Result<()> {
    if !is_ascii_only(serial) || serial.is_empty() || (serial.len() > 20) {
        Err(Error::InvalidSerial(serial.to_string()).into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_nqn() -> Result<()> {
        let valid_nqn = "nqn.2023-11.sh.tty:unit-tests";
        assert_valid_nqn(valid_nqn)?;

        // Not ASCII.
        assert!(assert_valid_nqn("nqn.2023-11.💩:invalid-nqn-unicode").is_err());
        // Too long.
        assert!(assert_valid_nqn("nqn.2023-11.sh.tty.foodreviews:Lopado\u{AD}temacho\u{AD}selacho\u{AD}galeo\u{AD}kranio\u{AD}leipsano\u{AD}drim\u{AD}hypo\u{AD}trimmato\u{AD}silphio\u{AD}karabo\u{AD}melito\u{AD}katakechy\u{AD}meno\u{AD}kichl\u{AD}epi\u{AD}kossypho\u{AD}phatto\u{AD}perister\u{AD}alektryon\u{AD}opte\u{AD}kephallio\u{AD}kigklo\u{AD}peleio\u{AD}lagoio\u{AD}siraio\u{AD}baphe\u{AD}tragano\u{AD}pterygon").is_err());

        assert_valid_subsys_name(valid_nqn)?;
        // Can't use discovery NQN.
        assert!(assert_valid_subsys_name("nqn.2014-08.org.nvmexpress.discovery").is_err());

        assert_compliant_nqn(valid_nqn)?;
        // Doesn't start with nqn.
        assert!(assert_compliant_nqn("blergh").is_err());
        // UUID prefix is not UUID.
        assert!(assert_compliant_nqn("nqn.2014-08.org.nvmexpress:uuid:42").is_err());
        // UUID prefix is valid UUID.
        assert_compliant_nqn(
            "nqn.2014-08.org.nvmexpress:uuid:39cd48a6-dee4-4eaa-a415-4e21e7a789f9",
        )?;

        Ok(())
    }

    #[test]
    fn test_valid_model() -> Result<()> {
        assert_valid_model("Dumb-O-Tron 2000")?;
        // Not ASCII-only
        assert!(assert_valid_model("💩").is_err());
        // Empty
        assert!(assert_valid_model("").is_err());
        // Too long.
        assert!(assert_valid_model("I am running out of dumb things to write!").is_err());

        Ok(())
    }
    #[test]
    fn test_valid_serial() -> Result<()> {
        assert_valid_model("1D10T")?;
        // Not ASCII-only
        assert!(assert_valid_serial("💩").is_err());
        // Empty
        assert!(assert_valid_serial("").is_err());
        // Too long.
        assert!(assert_valid_serial("dumb, but long enough").is_err());

        Ok(())
    }

    #[test]
    fn test_valid_nsid() -> Result<()> {
        assert_valid_nsid(1)?;

        // Can't use 0.
        assert!(assert_valid_nsid(0).is_err());
        // Can't use 0xffff_ffff.
        assert!(assert_valid_nsid(0xffff_ffff).is_err());

        Ok(())
    }
}
