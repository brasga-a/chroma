use chroma::{Engine, EngineError, Layout, MAX_JSON_LINE_BYTES, Response, Scene};
use std::io::{self, BufRead, Write};

enum InputLine {
    Bytes(Vec<u8>),
    TooLong,
    Eof,
}

fn read_bounded_line<R: BufRead>(reader: &mut R) -> io::Result<InputLine> {
    let mut line = Vec::with_capacity(1024);
    let mut too_long = false;

    loop {
        let (consumed, content_len, has_newline, chunk) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return if too_long {
                    Ok(InputLine::TooLong)
                } else if line.is_empty() {
                    Ok(InputLine::Eof)
                } else {
                    Ok(InputLine::Bytes(line))
                };
            }
            match available.iter().position(|byte| *byte == b'\n') {
                Some(index) => (index + 1, index, true, available[..index].to_vec()),
                None => (available.len(), available.len(), false, available.to_vec()),
            }
        };
        if !too_long {
            if line.len().saturating_add(content_len) > MAX_JSON_LINE_BYTES {
                line.clear();
                too_long = true;
            } else {
                line.extend_from_slice(&chunk);
            }
        }
        reader.consume(consumed);
        if has_newline {
            return if too_long {
                Ok(InputLine::TooLong)
            } else {
                Ok(InputLine::Bytes(line))
            };
        }
    }
}

fn write_response<W: Write>(writer: &mut W, response: &Response) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, response)?;
    writer.write_all(b"\n")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let layout = Layout::geometric_6x22_template();
    let scene = Scene::new("empty", Vec::new());
    let mut engine = Engine::new(layout, scene)?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());

    loop {
        let response = match read_bounded_line(&mut input)? {
            InputLine::Eof => break,
            InputLine::TooLong => Response::failure(EngineError::new(
                "request_too_large",
                format!("JSON line exceeds {MAX_JSON_LINE_BYTES} bytes"),
            )),
            InputLine::Bytes(bytes) => match serde_json::from_slice(&bytes) {
                Ok(request) => engine.handle_request(request),
                Err(error) => Response::failure(EngineError::new(
                    "invalid_json",
                    format!("request is not valid protocol JSON: {error}"),
                )),
            },
        };
        write_response(&mut output, &response)?;
        output.flush()?;
    }
    Ok(())
}
