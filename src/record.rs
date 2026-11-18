//! Terminal recordings: a pane's output as an asciinema recording
//! (asciicast v2: a JSON header line, then one [seconds, "o", text] line per
//! chunk of output and [seconds, "r", "COLSxROWS"] when it's resized), for
//! `asciinema play`, asciinema.org, or agg to make a GIF.

use std::io::Write;
use std::time::Instant;

pub struct Recorder {
    file: std::io::BufWriter<std::fs::File>,
    start: Instant,
    /// The end of a UTF-8 character cut off by the last chunk.
    carry: Vec<u8>,
    pub path: String,
}

impl Recorder {
    /// A new recording at `path` (~ for home), `cols` x `rows`, starting
    /// with what's on the screen now (`screen`: as VT sequences).
    pub fn start(path: &str, cols: u16, rows: u16, screen: &str) -> Option<Recorder> {
        let path = match path.strip_prefix("~/") {
            Some(rest) => format!("{}/{rest}", std::env::var("HOME").unwrap_or_default()),
            None => path.to_string(),
        };
        if let Some(dir) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let f = std::fs::File::create(&path).ok()?;
        let mut r = Recorder { file: std::io::BufWriter::new(f), start: Instant::now(), carry: Vec::new(), path };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let header = serde_json::json!({
            "version": 2, "width": cols, "height": rows, "timestamp": now,
            "env": {"TERM": "xterm-256color", "SHELL": std::env::var("SHELL").unwrap_or_default()},
            "title": "Tessera",
        });
        writeln!(r.file, "{header}").ok()?;
        if !screen.is_empty() {
            r.event("o", screen);
        }
        Some(r)
    }

    fn event(&mut self, kind: &str, text: &str) {
        let t = self.start.elapsed().as_secs_f64();
        let _ = writeln!(self.file, "[{t:.6}, {}, {}]", serde_json::Value::from(kind), serde_json::Value::from(text));
    }

    /// Output from the shell.
    pub fn output(&mut self, data: &[u8]) {
        let mut bytes = std::mem::take(&mut self.carry);
        bytes.extend_from_slice(data);
        // (Whole characters only; a cut-off one waits for the next chunk.)
        let cut = match std::str::from_utf8(&bytes) {
            Ok(_) => bytes.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(_) => bytes.len(),
        };
        self.carry = bytes[cut..].to_vec();
        let text = String::from_utf8_lossy(&bytes[..cut]).into_owned();
        if !text.is_empty() {
            self.event("o", &text);
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.event("r", &format!("{cols}x{rows}"));
    }

    pub fn finish(mut self) {
        let _ = self.file.flush();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn writes_asciicast() {
        let dir = std::env::temp_dir().join(format!("tessera-rec-{}", std::process::id()));
        let p = dir.join("a.cast");
        let mut r = super::Recorder::start(p.to_str().unwrap(), 80, 24, "\x1b[2J$ ").unwrap();
        let s = "héllo\r\n".as_bytes();
        r.output(&s[..2]); // (the é cut in two)
        r.output(&s[2..]);
        r.resize(100, 30);
        r.finish();
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[1][2], "\x1b[2J$ ");
        let out: String = lines[2..].iter().filter(|l| l[1] == "o").map(|l| l[2].as_str().unwrap().to_string()).collect();
        assert_eq!(out, "héllo\r\n");
        assert_eq!(lines.last().unwrap()[2], "100x30");
        let _ = std::fs::remove_dir_all(dir);
    }
}
