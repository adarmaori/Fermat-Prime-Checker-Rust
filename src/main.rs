use std::clone::Clone;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write, Read};
use num_bigint::BigUint;
use num_traits::{One, Zero};

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1 Megabyte
// const CHUNK_SIZE: usize = 8; // 8 bytes, for testing purposes

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
        // eprintln!("Failed to read file: {}", e);
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

fn square_number(
    src_filename: &str,
    start_index: usize,
    size: usize,
    dst_filename: &str,
) -> io::Result<usize> {
    let end_index: usize = start_index + size;
    let mut final_size: usize = 0;
    let mut carry: BigUint = BigUint::zero();
    
    clear_file(dst_filename)?;

    for i in start_index..end_index {
        let chunk_i = read_chunk(src_filename, i)?;
        for j in i..end_index {
            let chunk_j = if i == j {
                chunk_i.clone()
            } else {
                read_chunk(src_filename, j)?
            };
            
            let write_index = i + j - start_index;
            let mut product = chunk_i.clone() * chunk_j.clone();
            if i != j {
                product += product.clone(); // TODO: probably a better way to do this
            }
            let mut previous = BigUint::zero();
            if write_index > final_size {
                final_size = write_index;
            }
            else {
                previous = read_chunk(dst_filename, write_index).unwrap_or(BigUint::zero());
            }
            
            let result = product.clone() + previous.clone() + carry.clone();
            let (lower, upper) = split_biguint(&result);
            write_chunk(dst_filename, write_index, &lower)?;
            carry = upper;
            
        }
        if carry != BigUint::zero() {
            let write_index = i + end_index - start_index;
            write_chunk(dst_filename, write_index, &carry)?;
            carry = BigUint::zero();
            if write_index > final_size {
                final_size = write_index;
            }
        }
    }
    
    
    
    Ok(final_size + 1)

}

// Helper function to split a BigUint into lower and upper parts based on CHUNK_SIZE
fn split_biguint(num: &BigUint) -> (BigUint, BigUint) {
    let byte_size = CHUNK_SIZE;
    let base = BigUint::from(1u8) << (byte_size * 8);
    let lower = num.clone() % &base;
    let upper = num.clone() / &base;
    (lower, upper)
}
fn main() {
    let n = 23; // The fermat index to be tested (the actual number would be 2^(2^n) + 1)
    let mut result = BigUint::zero();
    
    
    let mod_bits = 1 + (1 << n); // The number of bits in the fermat number
    let max_size = (mod_bits - 1) / (CHUNK_SIZE * 8); // The maximum number of chunks needed to store the operand
    

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
        result = read_number_in_chunks(start_index, size, dst_filename).unwrap();
        // Swap files in a safe way
        rename_file(dst_filename, temp_filename).unwrap();
        rename_file(src_filename, dst_filename).unwrap();
        rename_file(temp_filename, src_filename).unwrap();
        clear_file(dst_filename).unwrap();
        
        
        // Taking the mod
        // NOTE: We're writing directly to the source file here, make sure this doesn't destroy anything
        while size > max_size {
            println!("Modding");
            // take the most significant chunk of the number
            let msc = read_chunk(src_filename, size - 1).unwrap();
            let to_subtract = msc.clone() - BigUint::one();
            if to_subtract == BigUint::zero() {
                if size - max_size == 1 {
                    // Go over the number to make sure at least one chunk is not zero, apart from the msc
                    let mut zeros = true;
                    let mut i = 0;
                    while i < max_size {
                        let x = read_chunk(src_filename, size - 2 - i).unwrap();
                        if x != BigUint::zero() {
                            zeros = false;
                            break;
                        }
                        i += 1;
                    }
                    if zeros {
                        break;
                    }
                    else {
                        // Subtract 1 from the whole number, and take out the msc
                        let mut borrow = BigUint::zero();
                        let mut i = 0;
                        while i < max_size {
                            let mut x = read_chunk(src_filename, size - 2 - i).unwrap();
                            if x == BigUint::zero() {
                                x = MINUS_ONE.clone();
                                borrow = BigUint::one();
                            }
                            else {
                                x -= BigUint::one();
                                borrow = BigUint::zero();
                            }
                            write_chunk(src_filename, size - 2 - i, &x).unwrap();
                            if borrow == BigUint::zero() {
                                break;
                            }
                            i += 1;
                        }
                        write_chunk(src_filename, size - 1, &BigUint::zero()).unwrap();
                        size -= 1;
                        break;
                    }
                }
                else {
                    // The msc should be zero
                    write_chunk(src_filename, size - 1, &BigUint::zero()).unwrap();

                    let mut val = read_chunk(src_filename, size - 2).unwrap() + BigUint::one();

                    // The second msc should be -1
                    write_chunk(src_filename, size - 2, &MINUS_ONE).unwrap();


                    // Subtract 1 from the sub_from chunk
                    let mut borrow = BigUint::zero();
                    let mut i = 0;
                    while i == 0 || borrow != BigUint::zero() {
                        let mut x = read_chunk(src_filename, size - 2 + i - max_size).unwrap();
                        if i == 0 {
                            if x > val {
                                x -= val.clone();
                            }
                            else {
                                x = x.clone() + (BigUint::one() << (CHUNK_SIZE * 8)) - val.clone();
                                borrow = BigUint::one();
                            }
                        }
                        else {
                            if x == BigUint::zero() {
                                x = MINUS_ONE.clone();
                                borrow = BigUint::one();
                            }
                            else {
                                x -= BigUint::one();
                                borrow = BigUint::zero();
                            }
                        }
                        write_chunk(src_filename, size - 2 + i - max_size, &x).unwrap();
                    }
                }
            }
            else {
                write_chunk(src_filename, size - 1, &BigUint::one()).unwrap();
                
                let sub_from = size - max_size - 1;
                
                let mut borrow = BigUint::zero();
                let mut i = 0;
                while i == 0 || borrow != BigUint::zero() {
                    if i == size - 1 {
                        size -= 1
                    }
                    let mut x = read_chunk(
                        src_filename,
                        sub_from + i
                    ).unwrap();
                    if i == 0 {
                        if x > to_subtract {
                            x -= to_subtract.clone();
                            borrow = BigUint::zero();
                        }
                        else {
                            x = x.clone() + (BigUint::one() << (CHUNK_SIZE * 8)) - to_subtract.clone();
                            borrow = BigUint::one();
                        }
                    }
                    else {
                        if x == BigUint::zero() {
                            x = MINUS_ONE.clone();
                            borrow = BigUint::one();
                        }
                        else {
                            x -= BigUint::one();
                            borrow = BigUint::zero();
                        }
                    }
                    write_chunk(src_filename, sub_from + i, &x).unwrap();
                    i += 1;
                }
            }
        }
        
        result = read_number_in_chunks(0, size, src_filename).unwrap();
        println!("3^{} % f_n, size={}", 1u64 << counter, size);
    }
    println!("{}", result);
}