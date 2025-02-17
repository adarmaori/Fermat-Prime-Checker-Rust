use std::io;
mod block_iterator;
mod arithmetic;

fn main() -> io::Result<()> {
    let file1 = "numbers/large_number1.bin"; // update to your correct file paths
    let file2 = "numbers/large_number2.bin";
    let out = "numbers/sum.bin";

    arithmetic::add_numbers(file1, file2, out)?;
    println!("Addition complete. Result stored in {}", out);
    Ok(())
}