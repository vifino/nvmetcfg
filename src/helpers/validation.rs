use crate::errors::*;

pub(crate) fn is_ascii_only(data: &str) -> bool {
    for c in data.chars() {
        if !c.is_ascii() && !c.is_ascii_control() {
            return false;
        }
    }
    true
}

pub fn assert_valid_nqn(nqn: &str) -> Result<()> {
    if !is_ascii_only(&nqn) {
        Err(Error::NQNNotAscii(nqn.to_string()).into())
    } else {
        Ok(())
    }
}

pub fn assert_valid_model(model: &str) -> Result<()> {
    if !is_ascii_only(model) && !model.is_empty() && (model.len() <= 40) {
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
