use std::clone::Clone;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write, Read};
use num_bigint::BigUint;
use num_traits::{One, Zero};

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1 Megabyte
// const CHUNK_SIZE: usize = 1 * 1024; // 1 Kilobyte
// const CHUNK_SIZE: usize = 1; // 1 byte for testing


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
    let u32_per_block = ((CHUNK_SIZE as f64)  / 4.0).ceil() as usize;

    // Get the u64 digits (limbs) of the BigUint
    let u32_digits = num.to_u32_digits();

    // Calculate the start and end indices for the desired block
    let start = block_index * u32_per_block;
    let end = start + u32_per_block;

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
    let size = number_size(num);
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


fn rename_file(src_filename: &str, dst_filename: &str) -> io::Result<()> {
    std::fs::rename(src_filename, dst_filename)
}


fn clear_file(filename: &str) -> io::Result<()> {
    let _ = std::fs::remove_file(filename);
    let _ = File::create(filename)?;
    Ok(())
}

fn number_size(num: &BigUint) -> usize {
    let bytes_len = num.to_bytes_le().len();
    let size = (bytes_len + CHUNK_SIZE - 1) / CHUNK_SIZE;
    size
}

fn square_number( src_filename: &str, start_index: usize, size: usize, dst_filename: &str) -> io::Result<usize> {
    let end_index: usize = start_index + size;
    let mut final_size: usize = 0;
    let mut carry: BigUint = BigUint::zero();

    clear_file(dst_filename)?;

    let mut read_buffer = vec![0; CHUNK_SIZE];
    let mut write_buffer = vec![0; CHUNK_SIZE];

    for i in start_index..end_index {
        let chunk_i = read_chunk_with_buffer(src_filename, i, &mut read_buffer)?;
        for j in i..end_index {
            let chunk_j = if i == j {
                chunk_i.clone()
            } else {
                read_chunk_with_buffer(src_filename, j, &mut read_buffer)?
            };

            let write_index = i + j - start_index;
            let mut product = &chunk_i * &chunk_j;
            if i != j {
                product *= 2u32;
            }

            let previous = if write_index > final_size || write_index == 0 {
                BigUint::zero()
            } else {
                read_chunk_with_buffer(dst_filename, write_index, &mut read_buffer)?
            };

            let result = product + previous + &carry;
            let (lower, upper) = split_biguint(&result);
            write_chunk_with_buffer(dst_filename, write_index, &lower, &mut write_buffer)?;
            carry = upper;

            if write_index > final_size {
                final_size = write_index;
            }
        }
        if carry != BigUint::zero() {
            let write_index = i + end_index - start_index;
            let (lower, upper) = split_biguint(&carry);
            write_chunk_with_buffer(dst_filename, write_index, &lower, &mut write_buffer)?;
            write_chunk_with_buffer(dst_filename, write_index + 1, &upper, &mut write_buffer)?;
            carry = BigUint::zero();
            if write_index > final_size {
                final_size = write_index;
            }
        }
    }

    Ok(final_size + 1)
}


fn read_chunk_with_buffer(filename: &str, chunk_index: usize, buffer: &mut [u8]) -> io::Result<BigUint> {
    let mut file = File::open(filename)?;
    let offset = chunk_index * CHUNK_SIZE;
    file.seek(SeekFrom::Start(offset as u64))?;
    file.read_exact(buffer)?;
    Ok(BigUint::from_bytes_le(buffer))
}

fn write_chunk_with_buffer(filename: &str, chunk_index: usize, big_num: &BigUint, buffer: &mut [u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).create(true).open(filename)?;
    let offset = chunk_index * CHUNK_SIZE;
    file.seek(SeekFrom::Start(offset as u64))?;
    let data = big_num.to_bytes_le();
    buffer[..data.len()].copy_from_slice(&data);
    if data.len() < CHUNK_SIZE {
        buffer[data.len()..].fill(0);
    }
    file.write_all(buffer)?;
    Ok(())
}


fn split_biguint(num: &BigUint) -> (BigUint, BigUint) {
    let byte_size = CHUNK_SIZE;
    let base = BigUint::from(1u8) << (byte_size * 8);
    let lower = num.clone() % &base;
    let upper = num.clone() / &base;
    (lower, upper)
}
fn main() {
    let n = 3; // The fermat index to be tested (the actual number would be 2^(2^n) + 1)
    let mut result = BigUint::zero();
    
    let mod_bits = 1 + (1 << n); // The number of bits in the fermat number
    let mut max_size = (mod_bits - 1) / (CHUNK_SIZE * 8); // The maximum number of chunks needed to store the operand
    if max_size == 0 {
        max_size = 1;
    }
    println!("Max size = {:x}", max_size);
    

    let MINUS_ONE: BigUint = (BigUint::from(1u32) << (CHUNK_SIZE * 8)) - BigUint::one();
    let num = BigUint::from(3u64);
    let src_filename = "test1.dat";
    let dst_filename = "test2.dat";
    let temp_filename = "temp.dat";
    clear_file(src_filename).unwrap();
    clear_file(dst_filename).unwrap();

    // Testing the square_number function
    write_number_in_chunks(&num, 0, src_filename).unwrap();
    let mut size = 1;
    let start_index = 0;
    let mut counter = 0;
    loop {
        counter += 1;
        if counter == 1 << n {
            break;
        }
        size = square_number(src_filename, start_index, size, dst_filename).unwrap();
        // Swap files in a safe way
        rename_file(dst_filename, temp_filename).unwrap();
        rename_file(src_filename, dst_filename).unwrap();
        rename_file(temp_filename, src_filename).unwrap();
        clear_file(dst_filename).unwrap();
        
        

        // Taking the mod
        // NOTE: We're writing directly to the source file here, make sure this doesn't destroy anything
        size = modulo(max_size, &MINUS_ONE, src_filename, size);
        result = read_number_in_chunks(0, size, src_filename).unwrap();
        
        println!("3^(2^{}) % f_n, size={}", counter, size);
    }
    if size == 1 {
        result = read_chunk(src_filename, 0).unwrap();
        println!("Result: {}", result);
    } else {
        println!("Result: Too big to print");
    }
}

fn create_file(filename: &str) -> io::Result<()> {
    File::create(filename)?;
    Ok(())
}

fn clone_file(src_filename: &str, dst_filename: &str) -> io::Result<()> {
    let mut src_file = File::open(src_filename)?;
    let mut dst_file = File::create(dst_filename)?;
    let mut buffer = vec![0; CHUNK_SIZE];

    loop {
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        dst_file.write_all(&buffer[..bytes_read])?;
    }

    Ok(())
}
fn modulo(max_size: usize, MINUS_ONE: &BigUint, src_filename: &str, mut size: usize) -> usize {
    let mut size_after = size;
    while size_after > max_size {
        println!("Modding");
        // take the most significant chunk of the number
        let msc = read_chunk(src_filename, size_after - 1).unwrap();
        if msc == BigUint::zero() {
            size_after -= 1;
            continue;
        }
        let to_subtract = msc.clone() - BigUint::one();
        if to_subtract == BigUint::zero() {
            if size_after - max_size == 1 {
                // Go over the number to make sure at least one chunk is not zero, apart from the msc
                let mut zeros = true;
                let mut i = 0;
                while i < max_size {
                    let x = read_chunk(src_filename, size_after - 2 - i).unwrap();
                    if x != BigUint::zero() {
                        zeros = false;
                        break;
                    }
                    i += 1;
                }
                if zeros {
                    break;
                } else {
                    // Subtract 1 from the whole number, and take out the msc
                    let mut borrow = BigUint::zero();
                    let mut i = 0;
                    while i < max_size {
                        let mut x = read_chunk(src_filename, size_after - 2 - i).unwrap();
                        if x == BigUint::zero() {
                            x = MINUS_ONE.clone();
                            borrow = BigUint::one();
                        } else {
                            x -= BigUint::one();
                            borrow = BigUint::zero();
                        }
                        write_chunk(src_filename, size_after - 2 - i, &x).unwrap();
                        if borrow == BigUint::zero() {
                            break;
                        }
                        i += 1;
                    }
                    write_chunk(src_filename, size_after - 1, &BigUint::zero()).unwrap();
                    size_after -= 1;
                    break;
                }
            } else {
                // The msc should be zero
                write_chunk(src_filename, size_after - 1, &BigUint::zero()).unwrap();

                let mut val = read_chunk(src_filename, size_after - 2).unwrap() + BigUint::one();

                // The second msc should be -1
                write_chunk(src_filename, size_after - 2, &MINUS_ONE).unwrap();


                // Subtract 1 from the sub_from chunk
                let mut borrow = BigUint::zero();
                let mut i = 0;
                while i == 0 || borrow != BigUint::zero() {
                    let mut x = read_chunk(src_filename, size_after - 2 + i - max_size).unwrap();
                    if i == 0 {
                        if x > val {
                            x -= val.clone();
                        } else {
                            x = x.clone() + (BigUint::one() << (CHUNK_SIZE * 8)) - val.clone();
                            borrow = BigUint::one();
                        }
                    } else {
                        if x == BigUint::zero() {
                            x = MINUS_ONE.clone();
                            borrow = BigUint::one();
                        } else {
                            x -= BigUint::one();
                            borrow = BigUint::zero();
                        }
                    }
                    write_chunk(src_filename, size_after - 2 + i - max_size, &x).unwrap();
                    i += 1;
                }
            }
        } else {
            write_chunk(src_filename, size_after - 1, &BigUint::one()).unwrap();

            let sub_from = size_after - max_size - 1;

            let mut borrow = BigUint::zero();
            let mut i = 0;
            while i == 0 || borrow != BigUint::zero() {
                if i == size_after - 1 {
                    size_after -= 1
                }
                let mut x = read_chunk(
                    src_filename,
                    sub_from + i
                ).unwrap();
                if i == 0 {
                    if x > to_subtract {
                        x -= to_subtract.clone();
                        borrow = BigUint::zero();
                    } else {
                        x = x.clone() + (BigUint::one() << (CHUNK_SIZE * 8)) - to_subtract.clone();
                        borrow = BigUint::one();
                    }
                } else {
                    if x == BigUint::zero() {
                        x = MINUS_ONE.clone();
                        borrow = BigUint::one();
                    } else {
                        x -= BigUint::one();
                        borrow = BigUint::zero();
                    }
                }
                write_chunk(src_filename, sub_from + i, &x).unwrap();
                i += 1;
            }
        }
    }
    size_after
}