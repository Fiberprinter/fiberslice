use std::{io::BufReader, path::Path};

use crate::object::ObjectMesh;

#[derive(thiserror::Error, Debug)]
pub enum LoadError {
    #[error("File Not Found")]
    FileNotFound,
    #[error("Broken File")]
    BrokenFile,
}

pub trait Loader {
    fn load<P: AsRef<Path>>(&self, path: P) -> Result<ObjectMesh, LoadError>;
}

pub struct STLLoader;

impl Loader for STLLoader {
    fn load<P: AsRef<Path>>(&self, path: P) -> Result<ObjectMesh, LoadError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|_| LoadError::FileNotFound)?;

        let mut reader = BufReader::new(file);

        Ok(nom_stl::parse_stl(&mut reader)
            .map_err(|_| LoadError::BrokenFile)?
            .into())
    }
}
