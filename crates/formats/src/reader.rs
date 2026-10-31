use crate::{ErrorKind, Format, FormatError, Result, Tag};

/// Leitor sequencial com verificação de limites sobre uma fatia de bytes.
///
/// Leituras que falham não avançam a posição.
#[derive(Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    format: Format,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], format: Format) -> Self {
        Reader {
            data,
            pos: 0,
            format,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn error(&self, kind: ErrorKind) -> FormatError {
        FormatError {
            format: self.format,
            offset: self.pos,
            kind,
        }
    }

    fn eof(&self, wanted: usize) -> FormatError {
        self.error(ErrorKind::UnexpectedEof {
            wanted,
            available: self.remaining(),
        })
    }

    pub fn seek(&mut self, pos: usize) -> Result<()> {
        if pos > self.data.len() {
            return Err(self.error(ErrorKind::Invalid(format!(
                "seek to 0x{pos:x} past end (0x{:x})",
                self.data.len()
            ))));
        }
        self.pos = pos;
        Ok(())
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.bytes(n).map(|_| ())
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return Err(self.eof(n));
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let s = self.bytes(N)?;
        let mut a = [0; N];
        a.copy_from_slice(s);
        Ok(a)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub fn u16_le(&mut self) -> Result<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub fn u16_be(&mut self) -> Result<u16> {
        self.array().map(u16::from_be_bytes)
    }

    pub fn i16_le(&mut self) -> Result<i16> {
        self.array().map(i16::from_le_bytes)
    }

    pub fn i16_be(&mut self) -> Result<i16> {
        self.array().map(i16::from_be_bytes)
    }

    pub fn u32_le(&mut self) -> Result<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub fn u32_be(&mut self) -> Result<u32> {
        self.array().map(u32::from_be_bytes)
    }

    pub fn tag(&mut self) -> Result<Tag> {
        self.array().map(Tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorKind, Format};

    #[test]
    fn reads_both_endians() {
        let d = [0x12, 0x34, 0x56, 0x78];
        let mut r = Reader::new(&d, Format::Glob);
        assert_eq!(r.u16_le().unwrap(), 0x3412);
        assert_eq!(r.u16_be().unwrap(), 0x5678);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn reads_u32_and_signed() {
        let d = [
            0x78, 0x56, 0x34, 0x12, 0x12, 0x34, 0x56, 0x78, 0xff, 0xff, 0xff, 0xfe,
        ];
        let mut r = Reader::new(&d, Format::Glob);
        assert_eq!(r.u32_le().unwrap(), 0x1234_5678);
        assert_eq!(r.u32_be().unwrap(), 0x1234_5678);
        assert_eq!(r.i16_le().unwrap(), -1);
        assert_eq!(r.i16_be().unwrap(), -2);
    }

    #[test]
    fn eof_reports_offset_and_sizes() {
        let d = [1, 2, 3];
        let mut r = Reader::new(&d, Format::Ne);
        r.skip(2).unwrap();
        let e = r.u32_le().unwrap_err();
        assert_eq!(e.offset, 2);
        assert_eq!(e.format, Format::Ne);
        assert_eq!(
            e.kind,
            ErrorKind::UnexpectedEof {
                wanted: 4,
                available: 1
            }
        );
    }

    #[test]
    fn failed_read_does_not_advance() {
        let d = [1, 2, 3];
        let mut r = Reader::new(&d, Format::Ne);
        assert!(r.u32_le().is_err());
        assert_eq!(r.pos(), 0);
    }

    #[test]
    fn seek_past_end_fails() {
        let d = [1, 2, 3];
        let mut r = Reader::new(&d, Format::Ne);
        assert!(r.seek(4).is_err());
        assert!(r.seek(3).is_ok());
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn skip_past_end_fails() {
        let d = [1, 2, 3];
        let mut r = Reader::new(&d, Format::Ne);
        assert!(r.skip(4).is_err());
        assert_eq!(r.pos(), 0);
        assert!(r.skip(usize::MAX).is_err());
    }

    #[test]
    fn bytes_returns_slice_and_advances() {
        let d = [1, 2, 3];
        let mut r = Reader::new(&d, Format::Ne);
        assert_eq!(r.bytes(2).unwrap(), &[1, 2]);
        assert_eq!(r.pos(), 2);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn tag_reads_file_order() {
        let d = *b"WHAT";
        let mut r = Reader::new(&d, Format::Ne);
        assert_eq!(r.tag().unwrap().to_string(), "WHAT");
    }

    #[test]
    fn error_uses_current_position() {
        let d = [0; 8];
        let mut r = Reader::new(&d, Format::Chnk);
        r.seek(5).unwrap();
        let e = r.error(ErrorKind::Invalid("x".into()));
        assert_eq!(e.offset, 5);
        assert_eq!(e.to_string(), "chnk @0x5: invalid data: x");
    }
}
