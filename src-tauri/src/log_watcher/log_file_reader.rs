//! Line reader that follows a log file by path and reopens it after rotation.

use std::{
    fs::File,
    io::{self, BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use same_file::Handle;

pub(crate) struct LogFileReader {
    path: PathBuf,
    reader: BufReader<File>,
    // Windows keeps a renamed file's creation time for a new file with the same name,
    // so identity must come from the volume and file index, not timestamps.
    handle: Handle,
}

impl LogFileReader {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let handle = Handle::from_file(file.try_clone()?)?;
        Ok(Self {
            path: path.to_path_buf(),
            reader: BufReader::new(file),
            handle,
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
        let current = match Handle::from_path(&self.path) {
            Ok(handle) => handle,
            // mid-rotation, keep reading the old file until the new one appears
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };
        let position = self.reader.stream_position()?;
        if current != self.handle {
            debug!("Log file {} was rotated, reopening", self.path.display());
            *self = Self::open(&self.path)?;
            Ok(true)
        } else if current.as_file().metadata()?.len() < position {
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
