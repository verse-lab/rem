impl ObjectId {
    pub fn new_foo(buffer: &[u8]) -> Result<ObjectId, Error> {
        match buffer.len() {
            40 => match Self::bar(buffer) {
                RetBar::Ok(x) => x,
                RetBar::Return(x) => return x,
            },
            len => Err(Error::InvalidHexEncodingLength(len)),
        }
    }
    fn bar(
        buffer: &[u8],
    ) -> RetBar<RetBar<Result<ObjectId, Error>, Result<ObjectId, Error>>, Result<ObjectId, Error>>
    {
        let result = Ok(ObjectId::Sha1(
            match <[u8; 20]>::from_hex(buffer).map_err(|err| match err {
                hex::FromHexError::InvalidHexCharacter { c, index } => Error::Invalid { c, index },
                hex::FromHexError::OddLength | hex::FromHexError::InvalidStringLength => {
                    unreachable!("BUG: This is already checked")
                }
            }) {
                Ok(x) => x,
                Err(e) => return RetBar::Return(Err(e)),
            },
        ));
        RetBar::Ok(result)
    }
}
enum RetBar<A, B> {
    Ok(A),
    Return(B),
}
