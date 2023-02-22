use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
    rc::Rc,
};

/// Representation of a file system
pub trait FileSystem: Clone {
    /// Errors
    type FSError: std::fmt::Debug;

    /// Tests for the existence of a given file
    fn exists<P: AsRef<Path>>(&self, path: P) -> Result<bool, Self::FSError>;

    /// Reads a file, specified by a path into a string
    fn read<P: AsRef<Path>>(&self, filename: P) -> Result<String, Self::FSError>;

    /// Writes into a file specified by a path
    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        filename: P,
        contents: C,
    ) -> Result<(), Self::FSError>;
}

/// Wrapper over the underlying file system
#[derive(Copy, Clone)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    type FSError = std::io::Error;

    fn read<P: AsRef<Path>>(&self, filename: P) -> Result<String, Self::FSError> {
        std::fs::read_to_string(filename)
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        filename: P,
        contents: C,
    ) -> Result<(), Self::FSError> {
        std::fs::write(filename, contents)
    }

    fn exists<P: AsRef<Path>>(&self, path: P) -> Result<bool, Self::FSError> {
        std::fs::try_exists(path)
    }
}

#[derive(Clone)]
pub struct SymbolicFileSystem(Rc<RefCell<HashMap<String, String>>>);

fn path_to_str<'a, P: 'a + AsRef<Path>>(path: &'a P) -> &'a str {
    path.as_ref().to_str().unwrap_or("")
}

fn path_to_pathbuf<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut pathbuf = PathBuf::new();
    pathbuf.set_file_name(path.as_ref().as_os_str());
    pathbuf
}

fn canonicalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    match std::fs::canonicalize(&path) {
        Ok(path) => path,
        Err(_) => path_to_pathbuf(path),
    }
}

impl FileSystem for SymbolicFileSystem {
    type FSError = !;

    fn read<P: AsRef<Path>>(&self, filename: P) -> Result<String, Self::FSError> {
        let path = canonicalize_path(filename);
        match self.0.borrow().get(path_to_str(&path)) {
            Some(s) => Ok(s.clone()),
            None => Ok("".into()),
        }
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        filename: P,
        contents: C,
    ) -> Result<(), Self::FSError> {
        let path = canonicalize_path(filename);
        let filename = path_to_str(&path).into();
        let contents = String::from_utf8_lossy(contents.as_ref());
        self.0.borrow_mut().insert(filename, contents.to_string());
        Ok(())
    }

    fn exists<P: AsRef<Path>>(&self, path: P) -> Result<bool, Self::FSError> {
        let path = canonicalize_path(path);
        let path = path_to_str(&path);
        Ok(self.0.borrow().contains_key(path))
    }
}

impl Display for SymbolicFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SymbolicFileSystem {{")?;
        for (key, vl) in self.0.borrow().iter() {
            writeln!(f, "file \"{}\": {{|", key)?;
            writeln!(f, "{}", vl)?;
            writeln!(f, "|}}")?
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl SymbolicFileSystem {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let mut map = HashMap::new();
        let mut to_visit: Vec<PathBuf> = {
            let mut pathbuf = PathBuf::new();
            pathbuf.set_file_name(path.as_ref().as_os_str());
            vec![pathbuf]
        };

        while !to_visit.is_empty() {
            let path = to_visit.pop().unwrap();
            let metadata = std::fs::metadata(&path)?;
            if metadata.is_dir() {
                let dir = std::fs::read_dir(&path)?;
                for entry in dir.into_iter() {
                    let entry = entry?;
                    to_visit.push(entry.path());
                }
            } else {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => {
                        let path = std::fs::canonicalize(path)?;
                        let path = path_to_str(&path).into();

                        map.insert(path, contents);
                    }
                    _ => (),
                }
            }
        }
        Ok(SymbolicFileSystem(Rc::new(RefCell::new(map))))
    }

    pub fn get(&self, index: &str) -> String {
        let path: PathBuf = canonicalize_path(index);
        let path: &str = path_to_str(&path);

        self.0
            .borrow()
            .get(path)
            .map(|v| v.into())
            .unwrap_or("".into())
    }
}

#[derive(Debug, Clone)]
pub struct FileLoader<T: FileSystem>(T);
unsafe impl<T: FileSystem> Send for FileLoader<T> {}
unsafe impl<T: FileSystem> Sync for FileLoader<T> {}

impl<T: FileSystem> FileLoader<T> {
    pub fn from(t: &T) -> Self {
        FileLoader(t.clone())
    }
}

impl<T: FileSystem> rustc_span::source_map::FileLoader for FileLoader<T> {
    fn file_exists(&self, path: &Path) -> bool {
        log::debug!(
            "checking if {:?} exists -> {:?}",
            path,
            self.0.exists(path).unwrap_or(false)
        );
        self.0.exists(path).unwrap_or(false)
    }

    fn read_file(&self, path: &Path) -> std::io::Result<String> {
        log::debug!("reading -> {:?}", path);
        self.0
            .read(path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
    }
}
