//! Native messaging framing (a 32-bit little-endian length, then UTF-8 JSON) and the single-line
//! framing used on the pipe. Both enforce a size limit before allocating anything.

use std::io::{self, BufRead, Read, Write};

pub const MAX_MESSAGE: usize = 1024 * 1024;

pub fn read_native<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_MESSAGE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message too large",
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn write_native<W: Write>(w: &mut W, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_MESSAGE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message too large",
        ));
    }
    w.write_all(&(payload.len() as u32).to_le_bytes())?;
    w.write_all(payload)?;
    w.flush()
}

/// Reads one `\n`-terminated line of at most `MAX_MESSAGE` bytes (without the newline).
pub fn read_line_limited<R: BufRead>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut line = Vec::new();
    let mut limited = r.take(MAX_MESSAGE as u64 + 1);
    let n = limited.read_until(b'\n', &mut line)?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "connection closed",
        ));
    }
    if line.last() == Some(&b'\n') {
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        Ok(line)
    } else if line.len() > MAX_MESSAGE {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message too large",
        ))
    } else {
        // The peer closed without a newline; accept what arrived.
        Ok(line)
    }
}

pub fn write_line<W: Write>(w: &mut W, payload: &[u8]) -> io::Result<()> {
    if payload.contains(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "payload contains a newline",
        ));
    }
    w.write_all(payload)?;
    w.write_all(b"\n")?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn native_messages_round_trip() {
        let mut buf = Vec::new();
        write_native(&mut buf, br#"{"a":1}"#).unwrap();
        assert_eq!(&buf[..4], &7u32.to_le_bytes());
        assert_eq!(read_native(&mut Cursor::new(buf)).unwrap(), br#"{"a":1}"#);
    }

    #[test]
    fn oversized_native_message_is_refused_before_reading_it() {
        let header = ((MAX_MESSAGE + 1) as u32).to_le_bytes();
        let err = read_native(&mut Cursor::new(header.to_vec())).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(write_native(&mut Vec::new(), &vec![0u8; MAX_MESSAGE + 1]).is_err());
    }

    #[test]
    fn truncated_native_message_is_an_error_not_a_hang() {
        let mut data = 10u32.to_le_bytes().to_vec();
        data.extend_from_slice(b"short");
        assert_eq!(
            read_native(&mut Cursor::new(data)).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(read_native(&mut Cursor::new(vec![1, 2])).is_err());
    }

    #[test]
    fn lines_are_split_and_limited() {
        let mut c = Cursor::new(b"first\r\nsecond\n".to_vec());
        assert_eq!(read_line_limited(&mut c).unwrap(), b"first");
        assert_eq!(read_line_limited(&mut c).unwrap(), b"second");
        assert!(read_line_limited(&mut c).is_err());
        let huge = vec![b'x'; MAX_MESSAGE + 10];
        assert_eq!(
            read_line_limited(&mut Cursor::new(huge))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn a_payload_with_a_newline_cannot_be_smuggled_onto_the_pipe() {
        assert!(write_line(&mut Vec::new(), b"{\"a\":\n1}").is_err());
        let mut buf = Vec::new();
        write_line(&mut buf, b"{}").unwrap();
        assert_eq!(buf, b"{}\n");
    }
}
