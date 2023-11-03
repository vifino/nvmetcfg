use crate::errors::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub fn read_str<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim().to_string())
}

pub fn write_str<P: AsRef<Path>>(path: P, data: String) -> Result<()> {
    let mut file = File::create(path)?;
    let value_string = data + "\n";
    file.write_all(value_string.as_bytes())?;
    Ok(())
}
