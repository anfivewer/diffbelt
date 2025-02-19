use diffbelt_aligned_bytes::AlignedBytesError;

#[derive(Debug)]
pub enum BodyReadError {
    IO,
    SizeLimit,
    Align(AlignedBytesError),
}
