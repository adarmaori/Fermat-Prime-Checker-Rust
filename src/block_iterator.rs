use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// const BLOCK_SIZE: usize = 1024 * 1024; // 1MB
const BLOCK_SIZE: usize = 2; // 2 bytes

pub struct BlockIterator<R: Read> {
    reader: R,
}

impl<R: Read> BlockIterator<R> {
    pub fn new(reader: R) -> Self {
        BlockIterator { reader }
    }
}

impl<R: Read> Iterator for BlockIterator<R> {
    // Each iteration returns a Result containing a vector of bytes
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = vec![0u8; BLOCK_SIZE];
        match self.reader.read(&mut buffer) {
            Ok(0) => None, // End of file reached
            Ok(n) => {
                buffer.truncate(n);
                Some(Ok(buffer))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// Iterates over the given file in 1MB blocks.  
/// Pass the file path, and this function returns a BlockIterator over its contents.
pub fn iter_file_blocks<P: AsRef<Path>>(path: P) -> io::Result<BlockIterator<File>> {
    let file = File::open(path)?;
    Ok(BlockIterator::new(file))
}