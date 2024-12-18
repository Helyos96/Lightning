use std::{fs::File, io::{self, Read}};

pub fn read_file(name: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(name)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
