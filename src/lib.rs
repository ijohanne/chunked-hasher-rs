use anyhow::{ensure, Result};
use std::{
    io::{Read, Seek, SeekFrom},
    iter::Iterator,
    marker::PhantomData,
};
pub mod hashers;

/// Combination trait of Read + Seek
pub trait ReadAndSeek: Read + Seek {}
impl<'a, T: Read + Seek> ReadAndSeek for T {}

/// Chunked hasher instance
pub struct ChunkedHasher<'a, H> {
    /// The buffer we'll iterate over when doing the chunked hashing
    seekable_buffer: &'a mut dyn ReadAndSeek,
    /// Size of the chunks to use per read cycle
    chunk_size: u64,
    /// Next chunk index to process
    next_chunk: u64,
    /// How much data we've read so far
    read_data: u64,
    // Hint pertaining to the total stream size
    stream_size: u64,
    _marker: PhantomData<H>,
}

impl<'a, H: hashers::Hasher> ChunkedHasher<'a, H> {
    /// Instantiate a fixed size chunked hasher
    ///
    /// # Arguments
    /// * `buffer` - the buffer to hash
    /// * `stream_size` - as neither Read nor Seek implements the ability to get
    ///   the full size, we need to give this hint
    /// * `fixed_size` - fixed chunk size, the last chunk will contain the
    ///   remainder
    ///
    /// # Example
    ///
    /// ```
    /// use chunked_hasher::{hashers::sha2::Sha256Hasher, Chunk, ChunkedHasher};
    /// # use std::io::Cursor;
    /// # use anyhow::Result;
    /// # pub fn main() -> Result<()> {
    /// # const WORDSTRING: &str = "brainstormremuneratedisabilityexperiment";
    /// # let mut buffer: Cursor<&[u8]> = Cursor::new(WORDSTRING.as_bytes());
    /// let original_chunks: Vec<Chunk> =
    ///     ChunkedHasher::<Sha256Hasher>::fixed_chunks(&mut buffer, WORDSTRING.len() as u64, 10)?
    ///         .collect();
    /// # Ok(())
    /// # }
    /// ```
    pub fn fixed_chunks(
        buffer: &'a mut dyn ReadAndSeek,
        stream_size: u64,
        fixed_size: u64,
    ) -> Result<Self> {
        ensure!(stream_size > 0, "Stream size must be greater than zero");
        ensure!(fixed_size > 0, "Fixed size must be greater than zero");

        let chunk_size = if fixed_size <= stream_size {
            fixed_size
        } else {
            stream_size
        };

        Ok(Self {
            seekable_buffer: buffer,
            _marker: PhantomData,
            chunk_size,
            stream_size,
            read_data: 0,
            next_chunk: 0,
        })
    }

    /// Instantiate a dynamic size chunked hasher
    ///
    /// # Arguments
    /// * `buffer` - the buffer to hash
    /// * `stream_size` - as neither Read nor Seek implements the ability to get
    ///   the full size, we need to give this hint
    /// * `dynamic_amount` - amount of chunks to chunk into, if it's not
    ///   perfectly divisible the remainder will be in its own chunk
    ///
    /// # Example
    ///
    /// ```
    /// use chunked_hasher::{hashers::sha2::Sha256Hasher, Chunk, ChunkedHasher};
    /// # use std::io::Cursor;
    /// # use anyhow::Result;
    /// # pub fn main() -> Result<()> {
    /// # const WORDSTRING: &str = "brainstormremuneratedisabilityexperiment";
    /// # let mut buffer: Cursor<&[u8]> = Cursor::new(WORDSTRING.as_bytes());
    /// let original_chunks: Vec<Chunk> =
    ///     ChunkedHasher::<Sha256Hasher>::dynamic_chunks(&mut buffer, WORDSTRING.len() as u64, 4)?
    ///         .collect();
    /// # Ok(())
    /// # }
    /// ```
    pub fn dynamic_chunks(
        buffer: &'a mut dyn ReadAndSeek,
        stream_size: u64,
        dynamic_amount: u64,
    ) -> Result<Self> {
        ensure!(stream_size > 0, "Stream size must be greater than zero");
        ensure!(
            dynamic_amount > 0,
            "Dynamic amount must be greater than zero"
        );

        let chunk_size = if dynamic_amount <= stream_size {
            (stream_size - (stream_size % dynamic_amount)) / dynamic_amount
        } else {
            stream_size
        };

        Ok(Self {
            seekable_buffer: buffer,
            _marker: PhantomData,
            chunk_size,
            stream_size,
            read_data: 0,
            next_chunk: 0,
        })
    }

    /// Size of the chunks except for the last remainer chunk, if any of those
    pub fn chunk_size(&self) -> u64 {
        self.chunk_size
    }

    /// Amount of chunks we will expect to be produced
    pub fn chunk_count(&self) -> u64 {
        f64::ceil(self.stream_size as f64 / self.chunk_size as f64) as u64
    }
}

impl<'a, H: hashers::Hasher> Iterator for ChunkedHasher<'a, H> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Chunk> {
        if self.read_data >= self.stream_size {
            return None;
        }
        match self
            .seekable_buffer
            .seek(SeekFrom::Start(self.next_chunk * self.chunk_size))
        {
            Ok(_) => {
                self.next_chunk += 1;
                let mut buf = vec![0u8; self.chunk_size as usize];
                match self.seekable_buffer.read(&mut buf) {
                    Ok(read_bytes) => {
                        self.read_data += read_bytes as u64;
                        Some(Chunk {
                            index: self.next_chunk - 1,
                            size: read_bytes as u64,
                            hash: H::hash_bytes(&buf),
                        })
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
}

/// Representation of a chunk including its position and hashed value
pub struct Chunk {
    /// Index in the streamed data this chunk pertains to
    pub index: u64,
    /// Size of the chunk that was hashed
    pub size: u64,
    /// Hash of chunked data
    pub hash: Vec<u8>,
}

impl std::fmt::Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use hex::encode;
        write!(f, "{}/{}/{}", self.index, self.size, encode(&self.hash))
    }
}

impl PartialEq for Chunk {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.size == other.size && self.hash == other.hash
    }
}

#[cfg(test)]
mod tests {
    use super::{
        hashers::sha2::{Sha256Hasher, Sha512Hasher},
        *,
    };
    use anyhow::Result;
    use std::io::Cursor;
    // The tests will use the same two constant strings below, which is made up of
    // 10 letter lower-case words joined to one long line. The second string will
    // have two of the words replaced by 'x'.
    const WORDSTRING: &str = "brainstormremuneratedisabilityexperiment\
                              goalkeepervegetarianattachmentsystematic\
                              relaxationpermissiondifficultyconference\
                              revolutionassumptionallocationliterature\
                              inhabitantdependenceoccupationprotection\
                              hypothesisdisappointexcitementunpleasant\
                              temptationassessmentthoughtfulpresidency\
                              censorshipwildernessreluctanceacceptable\
                              houseplantinstrumentoverchargeconvulsion\
                              acceptancefastidiousredundancydecorative\
                              attractiontechnologyvegetationmotorcycle\
                              curriculumhypnothizestereotypefederation";

    const WORDSTRING_DIFF: &str = "brainstormremuneratedisabilityexperiment\
                              goalkeepervegetarianxxxxxxxxxxsystematic\
                              relaxationpermissiondifficultyconference\
                              revolutionassumptionallocationliterature\
                              inhabitantdependenceoccupationprotection\
                              hypothesisdisappointexcitementxxxxxxxxxx\
                              temptationassessmentthoughtfulpresidency\
                              censorshipwildernessreluctanceacceptable\
                              houseplantinstrumentoverchargeconvulsion\
                              acceptancefastidiousredundancydecorative\
                              attractiontechnologyvegetationmotorcycle\
                              curriculumhypnothizestereotypefederation";

    // This macro sets up the cursors needed for in-memory testing
    macro_rules! perform_test {
        ($f: ident, $hasher: ty, $chunker: ident, $chunk_size: expr) => {
            #[test]
            fn $f() -> Result<()> {
                let mut buff_one: Cursor<&[u8]> = Cursor::new(WORDSTRING.as_bytes());
                let mut buff_two: Cursor<&[u8]> = Cursor::new(WORDSTRING_DIFF.as_bytes());
                perform_chunking!(
                    $hasher,
                    $chunker,
                    $chunk_size,
                    buff_one,
                    WORDSTRING.len(),
                    buff_two,
                    WORDSTRING_DIFF.len()
                );
                Ok(())
            }
        };
    }

    // This macro sets up the file handles to be used for testing
    macro_rules! perform_test_file {
        ($f: ident, $hasher: ty, $chunker: ident, $chunk_size: expr) => {
            #[test]
            fn $f() -> Result<()> {
                use std::fs;
                let mut original_path = env!("CARGO_MANIFEST_DIR").to_owned();
                original_path.push_str("/test-data/original.txt");
                let metadata_original_file = fs::metadata(&original_path)?;
                let mut original_file = fs::File::open(original_path)?;
                let mut different_path = env!("CARGO_MANIFEST_DIR").to_owned();
                different_path.push_str("/test-data/newfile.txt");
                let metadata_different_file = fs::metadata(&different_path)?;
                let mut different_file = fs::File::open(different_path)?;
                perform_chunking!(
                    $hasher,
                    $chunker,
                    $chunk_size,
                    original_file,
                    metadata_original_file.len(),
                    different_file,
                    metadata_different_file.len()
                );
                Ok(())
            }
        };
    }

    // This macro does the actual testing, and it's separated out as it's generic for all tests
    macro_rules! perform_chunking {
        ($hasher: ty, $chunker: ident, $chunk_size: expr, $input_one: ident, $input_one_length: expr, $input_two: ident, $input_two_length: expr) => {
            let original_chunks: Vec<Chunk> = ChunkedHasher::<$hasher>::$chunker(
                &mut $input_one,
                $input_one_length as u64,
                $chunk_size,
            )?
            .collect();
            let different_chunks: Vec<Chunk> = ChunkedHasher::<$hasher>::$chunker(
                &mut $input_two,
                $input_two_length as u64,
                $chunk_size,
            )?
            .collect();
            assert!(original_chunks != different_chunks);
            assert_eq!(original_chunks.len(), different_chunks.len());
            let diffed_ones = original_chunks
                .iter()
                .filter(|element| !different_chunks.contains(element))
                .collect::<Vec<&Chunk>>();
            assert_eq!(diffed_ones.len(), 2);
            assert_eq!(diffed_ones.get(0).unwrap().index, 1);
            assert_eq!(diffed_ones.get(1).unwrap().index, 5);
        };
    }

    // The macro usage below uses the above macros to setup testing for SHA256/SHA512, dynamic/fixed length chunks, and file/cursor backing.
    perform_test!(
        compare_two_strings_fixed_sha256,
        Sha256Hasher,
        fixed_chunks,
        40
    );

    perform_test!(
        compare_two_strings_dynamic_sha256,
        Sha256Hasher,
        dynamic_chunks,
        12
    );

    perform_test!(
        compare_two_strings_fixed_sha512,
        Sha512Hasher,
        fixed_chunks,
        40
    );

    perform_test!(
        compare_two_strings_dynamic_sha512,
        Sha512Hasher,
        dynamic_chunks,
        12
    );

    perform_test_file!(
        compare_two_strings_fixed_sha256_file,
        Sha256Hasher,
        fixed_chunks,
        40
    );

    perform_test_file!(
        compare_two_strings_dynamic_sha256_file,
        Sha256Hasher,
        dynamic_chunks,
        12
    );

    perform_test_file!(
        compare_two_strings_fixed_sha512_file,
        Sha512Hasher,
        fixed_chunks,
        40
    );

    perform_test_file!(
        compare_two_strings_dynamic_sha512_file,
        Sha512Hasher,
        dynamic_chunks,
        12
    );
}
