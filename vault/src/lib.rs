use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use zip::result::InvalidPassword;

#[derive(Debug)]
pub struct Error;

pub fn open(path: &str, password: Option<&[u8]>) -> Result<Vec<u8>, Error> {
    let path = Path::new(path);

    match fs::read(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(_) => return Err(Error),
        Ok(bytes) => return Ok(bytes),
    };

    let mut reader = None;

    let mut builder = PathBuf::from(path);
    while builder.pop() {
        match File::open(&builder) {
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(_) => return Err(Error),
            Ok(file) => {
                reader = Some(file);
                break;
            },
        }
    }

    let reader = reader.ok_or(Error)?;
    if let Ok(suffix) = path.strip_prefix(&builder) {
        let suffix = suffix.to_str().ok_or(Error)?;
        let suffix = suffix.replace('\\', "/");

        let mut zip = zip::ZipArchive::new(reader).unwrap();
        let result = if let Some(password) = password {
            zip.by_name_decrypt(&suffix, password)
        } else {
            zip.by_name(&suffix).map(|file| Ok(file))
        };

        match result {
            Err(e) => {
                println!("Error reading file '{}', error: {}", suffix, e);
                return Err(Error);
            },
            Ok(Err(InvalidPassword)) => {
                println!("Invaid password when reading '{}'", suffix);
                return Err(Error);
            },
            Ok(Ok(mut file)) => {
                let mut buffer = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buffer).map_err(|_| Error)?;
                return Ok(buffer);
            },
        }
    } else {
        unreachable!();
    }
}