use std::io::{self, BufRead, BufReader, Read};

const BOM: &[u8] = b"\xef\xbb\xbf";

/// calls `each` with every line without its line end, or with `None` for a line over `max_len`
pub fn for_each_line(
    reader: impl Read,
    max_len: usize,
    mut each: impl FnMut(Option<&[u8]>),
) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    let mut first = true;
    while let Some(fits) = read_line(&mut reader, max_len, &mut line)? {
        let mut bytes = line.as_slice();
        if first {
            bytes = bytes.strip_prefix(BOM).unwrap_or(bytes);
            first = false;
        }
        each(fits.then_some(bytes));
    }
    Ok(())
}

// reads in chunks so a huge line never sits in memory as a whole
fn read_line(
    reader: &mut impl BufRead,
    max_len: usize,
    line: &mut Vec<u8>,
) -> io::Result<Option<bool>> {
    line.clear();
    let mut fits = true;
    let mut read_any = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(read_any.then_some(fits));
        }
        read_any = true;
        let end = available.iter().position(|&byte| byte == b'\n');
        let chunk = &available[..end.unwrap_or(available.len())];
        if fits && line.len() + chunk.len() <= max_len {
            line.extend_from_slice(chunk);
        } else {
            fits = false;
            line.clear();
        }
        let used = chunk.len() + usize::from(end.is_some());
        reader.consume(used);
        if end.is_some() {
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(fits));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &[u8], max_len: usize) -> Vec<Option<Vec<u8>>> {
        let mut out = Vec::new();
        for_each_line(input, max_len, |line| out.push(line.map(<[u8]>::to_vec))).unwrap();
        out
    }

    #[test]
    fn reads_lines_without_bom_or_line_ends() {
        assert_eq!(
            lines(b"\xef\xbb\xbfls\r\ngit status\n\nlast", 100),
            [
                Some(b"ls".to_vec()),
                Some(b"git status".to_vec()),
                Some(Vec::new()),
                Some(b"last".to_vec())
            ]
        );
        assert!(lines(b"", 100).is_empty());
    }

    #[test]
    fn skips_lines_over_the_limit() {
        let mut input = vec![b'x'; 101];
        input.extend_from_slice(b"\nls\n");
        assert_eq!(lines(&input, 100), [None, Some(b"ls".to_vec())]);
    }
}
