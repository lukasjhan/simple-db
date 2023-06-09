use crate::varient;

#[derive(Debug, Clone)]
enum ColumnType {
    Null,
    I8,
    I16,
    I24,
    I32,
    I48,
    I64,
    F64,
    Zero,
    One,
    Blob(usize),
    Text(usize),
}

impl From<u64> for ColumnType {
    fn from(value: u64) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::I8,
            2 => Self::I16,
            3 => Self::I24,
            4 => Self::I32,
            5 => Self::I48,
            6 => Self::I64,
            7 => Self::F64,
            8 => Self::Zero,
            9 => Self::One,
            n if n > 12 && n % 2 == 0 => Self::Blob((n as usize - 12) / 2),
            n if n > 13 && n % 2 == 1 => Self::Text((n as usize - 13) / 2),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ColumnValue<'page> {
    Null,
    I8(i64),
    I16(i64),
    I24(i64),
    I32(i64),
    I48(i64),
    I64(i64),
    F64(f64),
    Zero,
    One,
    Blob(&'page [u8]),
    Text(&'page [u8]),
}

impl<'page> ColumnValue<'page> {
    pub fn is_number(&self) -> bool {
        match self {
            ColumnValue::I8(_)
            | ColumnValue::I16(_)
            | ColumnValue::I24(_)
            | ColumnValue::I32(_)
            | ColumnValue::I48(_)
            | ColumnValue::I64(_)
            | ColumnValue::F64(_) => true,
            ColumnValue::Zero | ColumnValue::One => true,
            ColumnValue::Null => false,
            ColumnValue::Blob(_) | ColumnValue::Text(_) => false,
        }
    }
}

impl Into<i64> for ColumnValue<'_> {
    fn into(self) -> i64 {
        match self {
            ColumnValue::Null => 0,
            ColumnValue::I8(n)
            | ColumnValue::I16(n)
            | ColumnValue::I24(n)
            | ColumnValue::I32(n)
            | ColumnValue::I48(n)
            | ColumnValue::I64(n) => n,
            ColumnValue::F64(n) => n as i64,
            ColumnValue::Zero => 0,
            ColumnValue::One => 1,
            _ => panic!("Cannot convert to i64"),
        }
    }
}

impl<'page> std::fmt::Display for ColumnValue<'page> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnValue::Null => write!(f, "NULL"),
            ColumnValue::I8(n)
            | ColumnValue::I16(n)
            | ColumnValue::I24(n)
            | ColumnValue::I32(n)
            | ColumnValue::I48(n)
            | ColumnValue::I64(n) => write!(f, "{}", n),
            ColumnValue::F64(n) => write!(f, "{}", n),
            ColumnValue::Zero => write!(f, "0"),
            ColumnValue::One => write!(f, "1"),
            ColumnValue::Blob(content) => write!(f, "<BLOB {} bytes>", content.len()),
            ColumnValue::Text(content) => write!(f, "{}", String::from_utf8_lossy(content)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Record<'page> {
    pub rowid: i64,
    pub values: Vec<ColumnValue<'page>>,
}

macro_rules! read_n_bytes {
    ($t:ident, $payload:expr, $cursor:expr, $n:expr) => {{
        let mut bytes = [0; 8];
        bytes[(8 - $n)..].copy_from_slice(&$payload[$cursor..$cursor + $n]);
        $cursor += $n;
        $t::from_be_bytes(bytes)
    }};
}

impl<'page> Record<'page> {
    pub fn read(rowid: i64, payload: &'page [u8]) -> Self {
        let mut cursor = 0;
        let (header_size, offset) = varient::read(&payload[cursor..]);
        cursor += offset;

        let mut remaining_bytes = header_size as usize - offset;
        let mut columns = Vec::with_capacity(remaining_bytes);

        while remaining_bytes > 0 {
            let (column, offset) = varient::read(&payload[cursor..]);
            cursor += offset;
            remaining_bytes -= offset;
            columns.push(ColumnType::from(column as u64));
        }

        let mut values = Vec::with_capacity(columns.len());
        for column in columns.iter() {
            let value = match column {
                ColumnType::Null => ColumnValue::Null,
                ColumnType::I8 => ColumnValue::I8(read_n_bytes!(i64, payload, cursor, 1)),
                ColumnType::I16 => ColumnValue::I16(read_n_bytes!(i64, payload, cursor, 2)),
                ColumnType::I24 => ColumnValue::I24(read_n_bytes!(i64, payload, cursor, 3)),
                ColumnType::I32 => ColumnValue::I32(read_n_bytes!(i64, payload, cursor, 4)),
                ColumnType::I48 => ColumnValue::I48(read_n_bytes!(i64, payload, cursor, 6)),
                ColumnType::I64 => ColumnValue::I64(read_n_bytes!(i64, payload, cursor, 8)),
                ColumnType::F64 => ColumnValue::F64(read_n_bytes!(f64, payload, cursor, 8)),
                ColumnType::Zero => ColumnValue::Zero,
                ColumnType::One => ColumnValue::One,
                ColumnType::Blob(size) => {
                    let value = ColumnValue::Blob(&payload[cursor..(cursor + *size)]);
                    cursor += *size;
                    value
                }
                ColumnType::Text(size) => {
                    let value = ColumnValue::Text(&payload[cursor..(cursor + *size)]);
                    cursor += *size;
                    value
                }
            };
            values.push(value);
        }

        Record { values, rowid }
    }
}