use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write, Read};
use num_bigint::BigUint;

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1 Megabyte

fn read_chunk(filename: &str, chunk_index: usize) -> io::Result<BigUint> {
    let mut file = File::open(filename).map_err(|e| {
        eprintln!("Failed to open file: {}", e);
        e
    })?;
    let offset = chunk_index * CHUNK_SIZE;
    file.seek(SeekFrom::Start(offset as u64)).map_err(|e| {
        eprintln!("Failed to seek file: {}", e);
        e
    })?;
    let mut buffer = vec![0; CHUNK_SIZE];
    file.read_exact(&mut buffer).map_err(|e| {
        eprintln!("Failed to read file: {}", e);
        e
    })?;
    let big_num = BigUint::from_bytes_le(&buffer);
    Ok(big_num)
}

fn write_chunk(filename: &str, chunk_index: usize, big_num: &BigUint) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(filename)
        .map_err(|e| {
            eprintln!("Failed to open file: {}", e);
            e
        })?;
    let offset = chunk_index * CHUNK_SIZE;
    file.seek(SeekFrom::Start(offset as u64)).map_err(|e| {
        eprintln!("Failed to seek file: {}", e);
        e
    })?;
    let mut data = big_num.to_bytes_le();
    if data.len() < CHUNK_SIZE {
        data.resize(CHUNK_SIZE, 0); // Pad with zeros to ensure consistent chunk size
    }
    file.write_all(&data).map_err(|e| {
        eprintln!("Failed to write to file: {}", e);
        e
    })?;
    Ok(())
}


fn chunkify_number(num: &BigUint, block_index: usize) -> BigUint {
    // Define the number of u64 digits (limbs) in a 1 MB block
    let u32_per_mb = 1024 * 1024 / 4;

    // Get the u64 digits (limbs) of the BigUint
    let u32_digits = num.to_u32_digits();

    // Calculate the start and end indices for the desired block
    let start = block_index * u32_per_mb;
    let end = start + u32_per_mb;

    // Slice the u64_digits array to get the desired block
    let block_slice = if start < u32_digits.len() {
        &u32_digits[start..end.min(u32_digits.len())]
    } else {
        // If the block_index is out of range, return 0 as BigUint
        return BigUint::from(0u32);
    };

    // Convert the u32 slice back to a BigUint
    BigUint::from_slice(block_slice)
}

fn write_number_in_chunks(num: &BigUint, start_index: usize, filename: &str) -> io::Result<()>{
    let size = ((num.bits() as f64 / 8.0) / CHUNK_SIZE as f64).ceil() as usize;
    for i in 0..size {
        let chunk = chunkify_number(num, i);
        write_chunk(filename, start_index + i, &chunk)?
    }
    
    Ok(())
}

fn read_number_in_chunks(start_index: usize, size: usize, filename: &str) -> io::Result<BigUint>{
    let mut block_slice: Vec<u32> = Vec::new();
    for i in start_index..start_index + size {
        let chunk = read_chunk(filename, i)?;
        let mut digits = chunk.to_u32_digits();
        if digits.len() < CHUNK_SIZE / 4 {
            digits.resize(CHUNK_SIZE / 4, 0); // Pad with zeros to ensure consistent chunk size
        }
        block_slice.append(&mut digits);
    }
    
    let res = BigUint::from_slice(&*block_slice);
    Ok(res)
}


fn square_number(src_filename: &str, dst_filename: &str, start_index: usize, size: usize) -> io::Result<u64>{
    // Squares the number in chunks and write the result back to the same file
    // Designed to handle numbers larger than can be stored in memory at once
    let end_index = start_index + size;
    let mut final_size: u64 = 0; // Does it need to be this big?
    for i in start_index..end_index {
        for j in i..end_index {
            let n1 = read_chunk(src_filename, i).unwrap();
            let n2 = read_chunk(src_filename, j).unwrap();
            let write_index = i + j - start_index; 
            // This functionality means that the result will be written as if start_index is the actual start of the number
            
            let result = n1 * n2;
            let res_size = ((result.bits() as f64 / 8.0) / CHUNK_SIZE as f64).ceil() as usize;
            for i in 0..res_size {
                let chunk = chunkify_number(&result, i);
                let previous = read_chunk(dst_filename, write_index + i).unwrap();
                write_chunk(dst_filename, write_index + i, &(chunk + previous)).unwrap();
            }
            
            if i == j && i == end_index - 1{
                final_size = (write_index + res_size) as u64;
            }
        }
    }
    Ok(final_size)
}


fn rename_file(src_filename: &str, dst_filename: &str) -> io::Result<()> {
    std::fs::rename(src_filename, dst_filename)
}


fn main() {

}