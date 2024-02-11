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

pub fn assert_compliant_nqn(nqn: &str) -> Result<()> {
    assert_valid_nqn(nqn)?;
    if !nqn.starts_with("nqn.") {
        Err(Error::NQNMissingNQN(nqn.to_string()).into())
    } else if nqn.len() < 15 {
        Err(Error::NQNTooShort(nqn.to_string()).into())
    } else if let Some(uuid) = nqn.strip_prefix("nqn.2014-08.org.nvmexpress:uuid:") {
        // NQN is a UUID. So we should ensure it's valid.
        if Uuid::try_parse(uuid).is_err() {
            Err(Error::NQNUuidInvalid(uuid.to_string()).into())
        } else {
            Ok(())
        }
    } else if nqn == "nqn.2014-08.org.nvmexpress.discovery" {
        Err(Error::CantCreateDiscovery.into())
    } else {
        // TODO: check if nqn has nqn.yyyy-mm, some reverse domain and a colon.
        // we can't make many other assumptions.
        let nqn_bytes = nqn.as_bytes();
        let has_dots_and_dash =
            (nqn_bytes[3] == b'.') && (nqn_bytes[8] == b'-') && (nqn_bytes[11] == b'.');
        let valid_date = nqn[4..8].parse::<i16>().is_ok() && nqn[9..10].parse::<i16>().is_ok();
        if !has_dots_and_dash || !valid_date {
            Err(Error::NQNInvalidDate(nqn.to_string()).into())
        } else {
            if let Some((domain, identifier)) = nqn[12..].split_once(":") {
                if domain == "org.nvmexpress" {
                    return Err(Error::NQNInvalidDomain(nqn.to_string()).into());
                }
                if !domain.is_empty() && !identifier.is_empty() {
                    return Ok(());
                }
            }
            Err(Error::NQNInvalidIdentifier(nqn.to_string()).into())
        }
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
        assert!(assert_valid_nqn("nqn.2023-11.sh.tty.foodreviews:Lopado-temacho-selacho-galeo-kranio-leipsano-drim-hypo-trimmato-silphio-karabo-melito-katakechy-meno-kichl-epi-kossypho-phatto-perister-alektryon-opte-kephallio-kigklo-peleio-lagoio-siraio-baphe-tragano-pterygon").is_err());

        Ok(())
    }

    #[test]
    fn test_compliant_nqn() -> Result<()> {
        let valid_nqn = "nqn.2023-11.sh.tty:unit-tests";

        assert_compliant_nqn(valid_nqn)?;
        // Doesn't start with nqn.
        assert!(assert_compliant_nqn("blergh").is_err());
        // Incorrect date formatting.
        assert!(assert_compliant_nqn("nqn.23_11.sh.tty:unit-tests").is_err());
        // Incorrect date digits.
        assert!(assert_compliant_nqn("nqn.abcd-ef.sh.tty:unit-tests").is_err());
        // No domain/identifier.
        assert!(assert_compliant_nqn("nqn.2023-11.a").is_err());
        // No domain/identifier.
        assert!(assert_compliant_nqn("nqn.2023-11.apple:").is_err());
        // No domain/identifier.
        assert!(assert_compliant_nqn("nqn.2023-11.:banana").is_err());

        // No discovery.
        assert!(assert_compliant_nqn("nqn.2014-08.org.nvmexpress.discovery").is_err());

        // org.nvmexpress
        assert!(assert_compliant_nqn("nqn.2023-11.org.nvmexpress:blah").is_err());

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
