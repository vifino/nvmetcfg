use crate::errors::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub(crate) fn read_str<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim().to_string())
}

pub(crate) fn write_str<P: AsRef<Path>, D: std::fmt::Display>(path: P, data: D) -> Result<()> {
    let mut file = File::create(path)?;
    write!(file, "{}\n", data)?;
    Ok(())
}
