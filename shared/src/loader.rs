use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use crate::object::ObjectMesh;

#[derive(thiserror::Error, Debug)]
pub enum LoadError {
    #[error("File Not Found")]
    FileNotFound,
    #[error("Broken File")]
    BrokenFile,
}

pub trait FileLoader {
    fn load<P: AsRef<Path>>(&self, path: P) -> Result<ObjectMesh, LoadError>;
}

pub trait BytesLoader {
    fn load_from_bytes(&self, bytes: &[u8]) -> Result<ObjectMesh, LoadError>;
}

pub struct STLLoader;

impl FileLoader for STLLoader {
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

impl BytesLoader for STLLoader {
    fn load_from_bytes(&self, bytes: &[u8]) -> Result<ObjectMesh, LoadError> {
        let mut reader = BufReader::new(Cursor::new(bytes));

        Ok(nom_stl::parse_stl(&mut reader)
            .map_err(|_| LoadError::BrokenFile)?
            .into())
    }
}
