//! Line reader that follows a log file by path and reopens it after rotation.

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(not(unix))]
use std::time::SystemTime;
use std::{
    fs::{metadata, File, Metadata},
    io::{self, BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[derive(PartialEq, Eq)]
struct FileId {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(not(unix))]
    created: Option<SystemTime>,
}

impl FileId {
    fn new(meta: &Metadata) -> Self {
        #[cfg(unix)]
        {
            Self {
                dev: meta.dev(),
                ino: meta.ino(),
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                created: meta.created().ok(),
            }
        }
    }
}

pub(crate) struct LogFileReader {
    path: PathBuf,
    reader: BufReader<File>,
    id: FileId,
}

impl LogFileReader {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let id = FileId::new(&file.metadata()?);
        Ok(Self {
            path: path.to_path_buf(),
            reader: BufReader::new(file),
            id,
        })
    }

    /// Like [`BufRead::read_line`], but on EOF switches to a rotated or truncated file.
    pub(crate) fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        let size = self.reader.read_line(buf)?;
        if size == 0 && self.reopen_if_rotated()? {
            self.reader.read_line(buf)
        } else {
            Ok(size)
        }
    }

    fn reopen_if_rotated(&mut self) -> io::Result<bool> {
        let meta = match metadata(&self.path) {
            Ok(meta) => meta,
            // Mid-rotation; keep the old file for now.
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };
        let position = self.reader.stream_position()?;
        if FileId::new(&meta) != self.id {
            debug!("Log file {} was rotated, reopening", self.path.display());
            *self = Self::open(&self.path)?;
            Ok(true)
        } else if meta.len() < position {
            debug!("Log file {} was truncated, rewinding", self.path.display());
            self.reader.seek(SeekFrom::Start(0))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::rename, io::Write};

    use super::*;

    fn append(path: &Path, text: &str) {
        let mut file = File::options()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write_all(text.as_bytes()).unwrap();
    }

    fn read_all(reader: &mut LogFileReader) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 0 {
            lines.push(line.clone());
            line.clear();
        }
        lines
    }

    #[test]
    fn follows_appends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        append(&path, "one\n");
        let mut reader = LogFileReader::open(&path).unwrap();
        assert_eq!(read_all(&mut reader), ["one\n"]);
        append(&path, "two\n");
        assert_eq!(read_all(&mut reader), ["two\n"]);
    }

    #[test]
    fn follows_rename_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        append(&path, "one\n");
        let mut reader = LogFileReader::open(&path).unwrap();
        assert_eq!(read_all(&mut reader), ["one\n"]);

        append(&path, "two\n");
        rename(&path, dir.path().join("test.1.log")).unwrap();
        assert_eq!(read_all(&mut reader), ["two\n"]);

        append(&path, "three\n");
        assert_eq!(read_all(&mut reader), ["three\n"]);
    }

    #[test]
    fn follows_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        append(&path, "one two three\n");
        let mut reader = LogFileReader::open(&path).unwrap();
        assert_eq!(read_all(&mut reader), ["one two three\n"]);

        File::create(&path).unwrap();
        append(&path, "four\n");
        assert_eq!(read_all(&mut reader), ["four\n"]);
    }
}
