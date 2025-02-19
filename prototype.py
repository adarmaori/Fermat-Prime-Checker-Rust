import os

CHUNK_SIZE = 1  # 1 Byte per block
BLOCK_MASK = (1 << (CHUNK_SIZE * 8)) - 1  # Mask for truncating to block size

def read_block(file, pos):
    """Reads a single CHUNK_SIZE block from the file at a given position."""
    global reads
    reads += 1
    file.seek(pos * CHUNK_SIZE)
    block = file.read(CHUNK_SIZE)
    return int.from_bytes(block, "little") if block else 0

def write_block(file, pos, value):
    """Writes a single CHUNK_SIZE block to the file at a given position."""
    global writes
    writes += 1
    file.seek(pos * CHUNK_SIZE)
    file.write(value.to_bytes(CHUNK_SIZE, "little"))

def get_file_size(file):
    """Gets the number of CHUNK_SIZE blocks in the file."""
    file.seek(0, os.SEEK_END)
    return file.tell() // CHUNK_SIZE

def square(input_filename, output_filename):
    """Squares a number stored in a file, minimizing disk accesses."""
    with open(input_filename, "rb") as infile, open(output_filename, "wb+") as outfile:
        size = get_file_size(infile)

        for i in range(size):
            block_i = read_block(infile, i)  # Read only one block
            carry = 0

            for j in range(i, size):
                if i != j:
                    block_j = 2 * read_block(infile, j)
                else:
                    block_j = block_i

                # Read current value from output file
                existing = read_block(outfile, i + j)

                # Compute product and handle carry
                block_prod = block_i * block_j + carry + existing
                product = block_prod & BLOCK_MASK
                carry = block_prod >> (CHUNK_SIZE * 8)

                # Write result to output file
                write_block(outfile, i + j, product)

            # Propagate remaining carry
            k = i + size
            while carry > 0:
                existing = read_block(outfile, k)
                new_value = existing + carry
                carry = new_value >> (CHUNK_SIZE * 8)
                new_value &= BLOCK_MASK
                write_block(outfile, k, new_value)
                k += 1



writes = 0
reads = 0
def file_to_num(filename):
    """Converts a file-based number representation back to an integer."""
    with open(filename, "rb") as f:
        size = get_file_size(f)
        res = 0
        for i in range(size):
            res += read_block(f, i) << (CHUNK_SIZE * 8 * i)
        return res

def num_to_file(n, filename):
    """Writes a number to a file in little-endian format."""
    with open(filename, "wb") as f:
        while n:
            f.write((n & BLOCK_MASK).to_bytes(CHUNK_SIZE, "little"))
            n >>= (CHUNK_SIZE * 8)

# Initial number (3)
num_to_file(3, "num.bin")

# Compute squares
for i in range(6):
    square("num.bin", "result.bin")
    os.replace("result.bin", "num.bin")  # Overwrite with new squared value
    print(f"3**{2**(i+1)} = {file_to_num('num.bin')}")
print(f"Total reads: {reads}, Total writes: {writes}")
