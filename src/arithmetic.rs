use std::fs::File;
use std::io::{self, Write, Read};
use std::cmp;

use crate::block_iterator;

/// A huge unsigned integer stored in a file.
pub struct HugeUint {
    pub file_path: String,
    /// The number of blocks this file occupies.
    pub num_blocks: usize,
}

impl HugeUint {
    pub fn new<S: Into<String>>(file_path: S, num_blocks: usize) -> Self {
        HugeUint {
            file_path: file_path.into(),
            num_blocks,
        }
    }

    /// Returns an iterator over this HugeUint's blocks.
    pub fn iter(&self) -> io::Result<block_iterator::BlockIterator<File>> {
        block_iterator::iter_file_blocks(&self.file_path)
    }
}

/// Adds two HugeUint numbers stored in files block-by-block and writes the sum to a new file.
pub fn add_huge_uints(a: &HugeUint, b: &HugeUint, out: &str) -> io::Result<HugeUint> {
    let mut iter1 = block_iterator::iter_file_blocks(&a.file_path)?;
    let mut iter2 = block_iterator::iter_file_blocks(&b.file_path)?;
    let mut out_file = File::create(out)?;
    let mut carry: u16 = 0;

    let mut result_size = cmp::max(&a.num_blocks, &b.num_blocks);

    loop {
        let block1 = match iter1.next() {
            Some(Ok(b)) => b,
            Some(Err(e)) => return Err(e),
            None => Vec::new(),
        };
        let block2 = match iter2.next() {
            Some(Ok(b)) => b,
            Some(Err(e)) => return Err(e),
            None => Vec::new(),
        };

        // If both blocks are empty, we have reached the end.
        if block1.is_empty() && block2.is_empty() {
            break;
        }

        // Process the blocks byte-by-byte.
        let max_len = cmp::max(block1.len(), block2.len());
        let mut out_block = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let a = block1.get(i).copied().unwrap_or(0) as u16;
            let b = block2.get(i).copied().unwrap_or(0) as u16;
            let sum = a + b + carry;
            out_block.push((sum & 0xFF) as u8);
            carry = sum >> 8;
        }
        out_file.write_all(&out_block)?;
    }

    // Write any remaining carry.
    if carry > 0 {
        out_file.write_all(&[carry as u8])?;
        result_size += 1;
    }
    out_file.flush()?;
    // Create a new HugeFile variable for the result and return it
    let result = HugeUint::new(out, result_size);
    Ok(result);
}

/// Writes a 128-bit number to a file in little-endian format (16 bytes).
pub fn write_number_file(path: &str, num: u128) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&num.to_le_bytes())?;
    file.flush()?;
    Ok(())
}

/// Reads a 128-bit number from a file (expects little-endian format, up to 16 bytes).
pub fn read_number_file(path: &str) -> io::Result<u128> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 16];
    let n = file.read(&mut buffer)?;
    // If the file is less than 16 bytes, the missing bytes are already zero.
    // You can also choose to return an error if n != 16.
    Ok(u128::from_le_bytes(buffer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    #[test]
    fn test_128bit_number_addition() -> io::Result<()> {
        // Prepare a temporary test directory.
        let base = PathBuf::from("numbers");
        fs::create_dir_all(&base)?;

        let file1 = base.join("test_large_number1.bin");
        let file2 = base.join("test_large_number2.bin");
        let out = base.join("test_sum.bin");

        // Known 128-bit numbers.
        // For this test we use 64-bit values extended into 128 bits.
        let num1: u64 = 0x123456789ABCDEF0;
        let num2: u64 = 0xFEDCBA9876543210;
        let expected_sum: u128 = (num1 as u128).wrapping_add(num2 as u128);
        println!("Expected sum: {:x}", expected_sum);

        // Write our test numbers as 128-bit little-endian values.
        write_number_file(file1.to_str().unwrap(), num1 as u128)?;
        write_number_file(file2.to_str().unwrap(), num2 as u128)?;

        // Create HugeUint objects.
        // Assuming the file was written as 16 bytes and that
        // block size is set to 2 bytes, we have 8 blocks per file.
        let huge1 = HugeUint::new(file1.to_str().unwrap(), 4);
        let huge2 = HugeUint::new(file2.to_str().unwrap(), 4);

        // Add the two numbers.
        add_huge_uints(&huge1, &huge2, out.to_str().unwrap())?;

        // Read the result.
        let result = read_number_file(out.to_str().unwrap())?;
        println!("Result: {:x}", result);

        // Clean up test files.
        fs::remove_file(file1)?;
        fs::remove_file(file2)?;
        fs::remove_file(out)?;

        assert_eq!(result, expected_sum);
        Ok(())
    }
}