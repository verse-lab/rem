impl ObjectId {
    pub fn new_foo(buffer: &[u8]) -> Result<ObjectId, Error> {
        match buffer.len() {
            40 => Self::bar(buffer),
            len => Err(Error::InvalidHexEncodingLength(len)),
        }
    }
    fn bar(buffer: &[u8]) -> RetBar<Result<ObjectId, Error>, Result<ObjectId, Error>> {
        Ok(ObjectId::Sha1(<[u8; 20]>::from_hex(buffer).map_err(
            |err| match err {
                hex::FromHexError::InvalidHexCharacter { c, index } => Error::Invalid { c, index },
                hex::FromHexError::OddLength | hex::FromHexError::InvalidStringLength => {
                    unreachable!("BUG: This is already checked")
                }
            },
        )?))
    }
}
