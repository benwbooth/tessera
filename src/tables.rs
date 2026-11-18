//! CSV and TSV files as tables to look through, however big: the file is
//! read into a temporary SQLite table in the background (its rows usable as
//! they arrive), and the viewer asks for one page of rows at a time, sorted,
//! filtered, its columns chosen and ordered, or by an SQL query of its own.
//!
//! The table is `t`, with a column per header (named as in the file, quoted
//! in SQL as "name") and `_row`, the line it came from.

use rusqlite::{params_from_iter, Connection};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

struct Table {
    path: String,
    db: PathBuf,
    columns: Vec<String>,
    /// Columns whose values all read as numbers (sorted as numbers).
    numeric: Vec<bool>,
    rows: u64,
    done: bool,
    error: String,
    size: u64,
    mtime: Option<std::time::SystemTime>,
}

fn tables() -> &'static Mutex<HashMap<String, Arc<Mutex<Table>>>> {
    static T: OnceLock<Mutex<HashMap<String, Arc<Mutex<Table>>>>> = OnceLock::new();
    T.get_or_init(Default::default)
}

fn delimiter(path: &str, first: &[u8]) -> u8 {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".tsv") || lower.ends_with(".tab") {
        return b'\t';
    }
    // (By what the first line has most of.)
    let line = first.split(|&b| b == b'\n').next().unwrap_or(&[]);
    let count = |c: u8| line.iter().filter(|&&b| b == c).count();
    [b',', b'\t', b';', b'|'].into_iter().max_by_key(|&c| count(c)).filter(|&c| count(c) > 0).unwrap_or(b',')
}

fn dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("tessera-tables")
}

/// Start reading a file (again if it changed): {"columns", "numeric", "rows",
/// "done", "size", "error"}, the rows read so far while it's reading.
pub fn open(path: &str) -> String {
    let (size, mtime) = match std::fs::metadata(path) {
        Ok(m) => (m.len(), m.modified().ok()),
        Err(e) => return json!({"columns": [], "rows": 0, "done": true, "error": e.to_string()}).to_string(),
    };
    let t = {
        let mut all = tables().lock().unwrap();
        let fresh = all.get(path).is_some_and(|t| {
            let t = t.lock().unwrap();
            t.size == size && t.mtime == mtime
        });
        if !fresh {
            let _ = std::fs::create_dir_all(dir());
            static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let db = dir().join(format!("{}-{n}.db", std::process::id()));
            if let Some(old) = all.get(path) {
                let _ = std::fs::remove_file(&old.lock().unwrap().db);
            }
            let t = Arc::new(Mutex::new(Table { path: path.to_string(), db, columns: Vec::new(), numeric: Vec::new(), rows: 0, done: false, error: String::new(), size, mtime }));
            all.insert(path.to_string(), t.clone());
            let t2 = t.clone();
            std::thread::spawn(move || {
                if let Err(e) = import(&t2) {
                    let mut t = t2.lock().unwrap();
                    t.error = e;
                    t.done = true;
                }
            });
        }
        all.get(path).cloned().unwrap()
    };
    let t = t.lock().unwrap();
    json!({"columns": t.columns, "numeric": t.numeric, "rows": t.rows, "done": t.done, "size": t.size, "error": t.error}).to_string()
}

fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn import(t: &Arc<Mutex<Table>>) -> Result<(), String> {
    let (path, db) = {
        let t = t.lock().unwrap();
        (t.path.clone(), t.db.clone())
    };
    let mut first = vec![0u8; 64 * 1024];
    let n = {
        use std::io::Read;
        std::fs::File::open(&path).and_then(|mut f| f.read(&mut first)).map_err(|e| e.to_string())?
    };
    first.truncate(n);
    let delim = delimiter(&path, &first);
    let mut rdr = csv::ReaderBuilder::new().delimiter(delim).flexible(true).has_headers(true).from_path(&path).map_err(|e| e.to_string())?;
    // (Headers: as in the file, made unique, "column N" for blank ones.)
    let mut seen: HashMap<String, usize> = HashMap::new();
    let columns: Vec<String> = rdr
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let base = if h.trim().is_empty() { format!("column {}", i + 1) } else { h.trim().to_string() };
            let k = seen.entry(base.to_lowercase()).or_insert(0);
            *k += 1;
            if *k > 1 { format!("{base} ({k})") } else { base }
        })
        .collect();
    let mut conn = Connection::open(&db).map_err(|e| e.to_string())?;
    let _ = conn.execute_batch("PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA temp_store=MEMORY;");
    let cols_sql: Vec<String> = columns.iter().map(|c| format!("{} TEXT", quote_ident(c))).collect();
    conn.execute(&format!("CREATE TABLE t (_row INTEGER PRIMARY KEY, {})", cols_sql.join(", ")), []).map_err(|e| e.to_string())?;
    {
        let mut tt = t.lock().unwrap();
        tt.columns = columns.clone();
        tt.numeric = vec![true; columns.len()];
    }
    let ncol = columns.len();
    let marks = vec!["?"; ncol + 1].join(", ");
    let insert = format!("INSERT INTO t VALUES ({marks})");
    let mut numeric = vec![true; ncol];
    let mut any = vec![false; ncol];
    let mut row: u64 = 0;
    let mut rec = csv::StringRecord::new();
    loop {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let mut batch = 0;
        {
            let mut st = tx.prepare_cached(&insert).map_err(|e| e.to_string())?;
            while batch < 20000 {
                match rdr.read_record(&mut rec) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(_) => continue,
                }
                row += 1;
                let mut vals: Vec<rusqlite::types::Value> = Vec::with_capacity(ncol + 1);
                vals.push(rusqlite::types::Value::Integer(row as i64));
                for i in 0..ncol {
                    let v = rec.get(i).unwrap_or("");
                    if !v.trim().is_empty() {
                        any[i] = true;
                        if numeric[i] && v.trim().parse::<f64>().is_err() {
                            numeric[i] = false;
                        }
                    }
                    vals.push(rusqlite::types::Value::Text(v.to_string()));
                }
                st.execute(params_from_iter(vals)).map_err(|e| e.to_string())?;
                batch += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        let mut tt = t.lock().unwrap();
        tt.rows = row;
        tt.numeric = numeric.iter().zip(&any).map(|(n, a)| *n && *a).collect();
        if batch < 20000 {
            tt.done = true;
            return Ok(());
        }
    }
}

/// A page of rows: spec {"columns": [names, in order], "sort": [[name,
/// desc]], "filter": text in any shown column, "query": an SQL condition
/// (WHERE ...) or a whole SELECT, "offset", "limit"} -> {"columns", "rows":
/// [[...]], "total", "error"}.
pub fn query(path: &str, spec: &str) -> String {
    let Some(t) = tables().lock().unwrap().get(path).cloned() else { return json!({"error": "Not read yet."}).to_string() };
    let (db, all_cols, numeric) = {
        let t = t.lock().unwrap();
        (t.db.clone(), t.columns.clone(), t.numeric.clone())
    };
    let spec: Value = serde_json::from_str(spec).unwrap_or_default();
    let offset = spec["offset"].as_u64().unwrap_or(0);
    let limit = spec["limit"].as_u64().unwrap_or(200).min(5000);
    let conn = match Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(e) => return json!({"error": e.to_string()}).to_string(),
    };
    let query = spec["query"].as_str().unwrap_or("").trim().to_string();
    let whole = query.to_ascii_lowercase().starts_with("select") || query.to_ascii_lowercase().starts_with("with");
    // The rows to page through.
    let (from, cols): (String, Vec<String>) = if whole {
        // (Its own columns, whatever it selects.)
        let from = format!("({})", query.trim_end_matches(';'));
        let cols = conn
            .prepare(&format!("SELECT * FROM {from} LIMIT 0"))
            .map(|s| s.column_names().map(|c| c.to_string()).collect())
            .unwrap_or_default();
        (from, cols)
    } else {
        let shown: Vec<String> = spec["columns"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).filter(|c| all_cols.contains(c)).collect()).filter(|v: &Vec<String>| !v.is_empty()).unwrap_or_else(|| all_cols.clone());
        ("t".to_string(), shown)
    };
    let mut wheres: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if !whole && !query.is_empty() {
        wheres.push(format!("({query})"));
    }
    let filter = spec["filter"].as_str().unwrap_or("").trim().to_string();
    if !filter.is_empty() && !cols.is_empty() {
        // (Every word somewhere in the row, any case.)
        for w in filter.split_whitespace() {
            let any: Vec<String> = cols.iter().map(|c| format!("{} LIKE ? ESCAPE '\\'", quote_ident(c))).collect();
            wheres.push(format!("({})", any.join(" OR ")));
            let pat = format!("%{}%", w.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            for _ in &cols {
                args.push(pat.clone());
            }
        }
    }
    let wh = if wheres.is_empty() { String::new() } else { format!(" WHERE {}", wheres.join(" AND ")) };
    let mut order: Vec<String> = Vec::new();
    for s in spec["sort"].as_array().into_iter().flatten() {
        let (Some(c), desc) = (s[0].as_str(), s[1].as_bool().unwrap_or(false)) else { continue };
        if !cols.iter().any(|x| x == c) {
            continue;
        }
        let num = all_cols.iter().position(|x| x == c).is_some_and(|i| numeric.get(i).copied().unwrap_or(false));
        let key = if num { format!("CAST({} AS REAL)", quote_ident(c)) } else { format!("{} COLLATE NOCASE", quote_ident(c)) };
        order.push(format!("{key} {}", if desc { "DESC" } else { "ASC" }));
    }
    if order.is_empty() && !whole {
        order.push("_row".into());
    }
    let ord = if order.is_empty() { String::new() } else { format!(" ORDER BY {}", order.join(", ")) };
    let select: Vec<String> = cols.iter().map(|c| quote_ident(c)).collect();
    let sql = format!("SELECT {} FROM {from}{wh}{ord} LIMIT {limit} OFFSET {offset}", if select.is_empty() { "*".into() } else { select.join(", ") });
    let total: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {from}{wh}"), params_from_iter(args.iter()), |r| r.get(0)).unwrap_or(-1);
    let mut st = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return json!({"columns": cols, "rows": [], "total": 0, "error": e.to_string()}).to_string(),
    };
    let n = st.column_count();
    let rows: Result<Vec<Vec<String>>, _> = st
        .query_map(params_from_iter(args.iter()), |r| {
            (0..n)
                .map(|i| {
                    Ok(match r.get_ref(i)? {
                        rusqlite::types::ValueRef::Null => String::new(),
                        rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                        rusqlite::types::ValueRef::Real(v) => v.to_string(),
                        rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                        rusqlite::types::ValueRef::Blob(b) => format!("<{} bytes>", b.len()),
                    })
                })
                .collect()
        })
        .and_then(|it| it.collect());
    match rows {
        Ok(rows) => json!({"columns": cols, "rows": rows, "total": total}).to_string(),
        Err(e) => json!({"columns": cols, "rows": [], "total": 0, "error": e.to_string()}).to_string(),
    }
}

/// Its temporary table goes (the card closed).
pub fn close(path: &str) {
    if let Some(t) = tables().lock().unwrap().remove(path) {
        let _ = std::fs::remove_file(&t.lock().unwrap().db);
    }
}

/// Whether a file is one this views (by its name).
pub fn is_table(path: &str) -> bool {
    let l = path.to_ascii_lowercase();
    l.ends_with(".csv") || l.ends_with(".tsv") || l.ends_with(".tab")
}

#[allow(dead_code)]
fn exists(p: &Path) -> bool {
    p.exists()
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_sorts_filters_queries() {
        let d = std::env::temp_dir().join(format!("tessera-table-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("people.csv");
        let mut body = String::from("name,age,city\n");
        for (n, a, c) in [("ann", 31, "Oslo"), ("bob", 9, "Rome"), ("cyd", 100, "Oslo"), ("dee", 42, "Lima")] {
            body.push_str(&format!("{n},{a},{c}\n"));
        }
        std::fs::write(&f, body).unwrap();
        let p = f.to_string_lossy().to_string();
        let mut v: serde_json::Value = serde_json::from_str(&super::open(&p)).unwrap();
        while !v["done"].as_bool().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            v = serde_json::from_str(&super::open(&p)).unwrap();
        }
        assert_eq!(v["rows"], 4);
        assert_eq!(v["numeric"][1], true);
        let q = |spec: &str| -> serde_json::Value { serde_json::from_str(&super::query(&p, spec)).unwrap() };
        // (Numbers sort as numbers: 9 before 31.)
        let r = q(r#"{"sort": [["age", false]]}"#);
        assert_eq!(r["rows"][0][0], "bob", "{r}");
        assert_eq!(r["rows"][3][0], "cyd");
        let r = q(r#"{"filter": "oslo", "columns": ["city", "name"]}"#);
        assert_eq!(r["total"], 2);
        assert_eq!(r["columns"][0], "city");
        let r = q(r#"{"query": "CAST(age AS INT) > 40"}"#);
        assert_eq!(r["total"], 2);
        let r = q(r#"{"query": "SELECT city, COUNT(*) AS n FROM t GROUP BY city ORDER BY n DESC"}"#);
        assert_eq!(r["rows"][0][0], "Oslo", "{r}");
        assert_eq!(r["columns"][1], "n");
        super::close(&p);
        let _ = std::fs::remove_dir_all(d);
    }
}
