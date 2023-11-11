use crate::errors::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub fn read_str<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim().to_string())
}

pub fn write_str<P: AsRef<Path>, D: std::fmt::Display>(path: P, data: D) -> Result<()> {
    let mut file = File::create(path)?;
    // Unfortunately, we need to write in a single write call.
    let value = format!("{data}");
    file.write_all(value.as_bytes())?;
    Ok(())
}
