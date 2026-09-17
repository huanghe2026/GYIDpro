//! 最小确定性 CBOR 编解码器（RFC 8949 §4.2 Core Deterministic Encoding Requirements）。
//!
//! TRIP 要求“同逻辑内容在任何合规实现上产生逐字节一致的编码”，因为签名与块哈希
//! 都直接覆盖 CBOR 原始字节（draft-04 §3.3 / §3.4）。因此 trip-core 不依赖通用
//! CBOR 库的排序行为，而是手写编码器并严格保证：
//!
//! - 仅使用 definite-length；
//! - map 的 key 全部为小整数（0..23，单字节首字节），按整数升序写入，
//!   等价于规范化要求的“编码长度升序、长度相同则字节升序”；
//! - 浮点一律以 float64（0xfb）输出；
//! - 整数取最小宽度编码。
//!
//! 同时提供一个仅供验证/测试使用的最小递归解析器 [`parse_all`]。

use crate::error::{Result, TripError};

/* ------------------------------------------------------------------ */
/* 编码器                                                              */
/* ------------------------------------------------------------------ */

/// 确定性 CBOR 字节流写入器。
#[derive(Debug, Clone, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// 创建空写入器。
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// 以预估容量创建。
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    /// 写入类型头（major type + argument）。
    fn head(&mut self, major: u8, n: u64) {
        let prefix = major << 5;
        if n < 24 {
            self.buf.push(prefix | n as u8);
        } else if n <= u8::MAX as u64 {
            self.buf.push(prefix | 24);
            self.buf.push(n as u8);
        } else if n <= u16::MAX as u64 {
            self.buf.push(prefix | 25);
            self.buf.extend_from_slice(&(n as u16).to_be_bytes());
        } else if n <= u32::MAX as u64 {
            self.buf.push(prefix | 26);
            self.buf.extend_from_slice(&(n as u32).to_be_bytes());
        } else {
            self.buf.push(prefix | 27);
            self.buf.extend_from_slice(&n.to_be_bytes());
        }
    }

    /// major 0：无符号整数。
    pub fn uint(&mut self, v: u64) -> &mut Self {
        self.head(0, v);
        self
    }

    /// major 2：字节串。
    pub fn bstr(&mut self, b: &[u8]) -> &mut Self {
        self.head(2, b.len() as u64);
        self.buf.extend_from_slice(b);
        self
    }

    /// major 4：数组头，随后写入 `n` 个值。
    pub fn array(&mut self, n: u64) -> &mut Self {
        self.head(4, n);
        self
    }

    /// major 5：map 头，随后写入 `n` 组 key/value。
    pub fn map(&mut self, n: u64) -> &mut Self {
        self.head(5, n);
        self
    }

    /// major 7：布尔（simple value 20/21）。
    pub fn bool(&mut self, b: bool) -> &mut Self {
        self.buf.push(if b { 0xf5 } else { 0xf4 });
        self
    }

    /// major 7：null（simple value 22）。
    pub fn null(&mut self) -> &mut Self {
        self.buf.push(0xf6);
        self
    }

    /// major 7：float64（附加信息 27）。
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.buf.push(0xfb);
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    /// 追加另一个写入器（或子结构）已规范化的原始字节。
    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    /// 返回已写入的字节。
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// 借用已写入的字节。
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }
}

/* ------------------------------------------------------------------ */
/* 解析器（仅覆盖 TRIP 用到的子集；非规范输入一律拒绝）                 */
/* ------------------------------------------------------------------ */

/// 最小 CBOR 值模型，供验证侧读取协议消息。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// major 0 无符号整数。
    UInt(u64),
    /// major 2 字节串。
    BStr(Vec<u8>),
    /// major 7 布尔。
    Bool(bool),
    /// major 7 null。
    Null,
    /// major 7 浮点（f32 在读取时提升为 f64）。
    Float(f64),
    /// major 4 数组。
    Array(Vec<Value>),
    /// major 5 map，保留写入顺序（协议要求 key 已排序）。
    Map(Vec<(Value, Value)>),
}

impl Value {
    /// 若为 [`Value::UInt`] 返回值。
    pub fn as_uint(&self) -> Result<u64> {
        match self {
            Value::UInt(v) => Ok(*v),
            other => Err(TripError::Cbor(format!("expected uint, got {other:?}"))),
        }
    }

    /// 若为 [`Value::BStr`] 返回字节。
    pub fn as_bstr(&self) -> Result<&[u8]> {
        match self {
            Value::BStr(b) => Ok(b),
            other => Err(TripError::Cbor(format!("expected bstr, got {other:?}"))),
        }
    }

    /// 若为 [`Value::Bool`] 返回值。
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(TripError::Cbor(format!("expected bool, got {other:?}"))),
        }
    }

    /// 若为 [`Value::Float`] 返回值。
    pub fn as_float(&self) -> Result<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            other => Err(TripError::Cbor(format!("expected float, got {other:?}"))),
        }
    }

    /// 若为 [`Value::Map`]，按整数 key 查表。
    pub fn field(&self, key: u64) -> Result<&Value> {
        match self {
            Value::Map(pairs) => pairs
                .iter()
                .find(|(k, _)| matches!(k, Value::UInt(i) if *i == key))
                .map(|(_, v)| v)
                .ok_or_else(|| TripError::Cbor(format!("missing map field {key}"))),
            other => Err(TripError::Cbor(format!("expected map, got {other:?}"))),
        }
    }
}

/// 解析单个 CBOR 值，返回值与剩余字节。
pub fn parse(input: &[u8]) -> Result<(Value, &[u8])> {
    let (v, rest) = parse_one(input)?;
    Ok((v, rest))
}

/// 切出第一个完整 CBOR 值的原始字节与剩余字节（不解释内容）。
///
/// 用于自分隔的 CBOR 帧流（例如 `POST /v1/evidence` 请求体是多条面包屑
/// 完整 CBOR map 的顺序拼接）：调用方可反复 `split_value` 取得每条的字节
/// 切片，再交给对应类型的 `from_cbor` 做规范化解析与验签。
///
/// 只支持本协议使用的确定性编码子集；遇到负整数、文本串、标签、
/// indefinite-length 或保留附加信息一律报错。
pub fn split_value(input: &[u8]) -> Result<(&[u8], &[u8])> {
    let len = value_len(input)?;
    if input.len() < len {
        return Err(TripError::Cbor(
            "unexpected end while skipping cbor value".into(),
        ));
    }
    Ok((&input[..len], &input[len..]))
}

/// 返回从 `input` 起始的一个完整 CBOR 值所占字节数（递归计入容器内容）。
fn value_len(input: &[u8]) -> Result<usize> {
    let (&first, rest) = input
        .split_first()
        .ok_or_else(|| TripError::Cbor("unexpected end of cbor input".into()))?;
    let major = first >> 5;
    let ai = first & 0x1f;

    // 头部（首字节 + 参数字节）长度。
    let arg_bytes = match ai {
        0..=23 => 0,
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => {
            return Err(TripError::Cbor(
                "indefinite/reserved cbor head not permitted".into(),
            ))
        }
    };
    if rest.len() < arg_bytes {
        return Err(TripError::Cbor(
            "unexpected end while reading cbor head argument".into(),
        ));
    }
    let head_len = 1 + arg_bytes;

    match major {
        // major 0 uint：仅头部。
        0 => Ok(head_len),
        // major 2 bstr：头部 + payload。
        2 => {
            let (payload_len, _) = read_arg(ai, rest)?;
            Ok(head_len + payload_len as usize)
        }
        // major 4 array：n 个值；major 5 map：2n 个值。
        4 | 5 => {
            let (count, _) = read_arg(ai, rest)?;
            let items = if major == 4 { count } else { count * 2 };
            let mut offset = head_len;
            for _ in 0..items {
                let sub = input.get(offset..).ok_or_else(|| {
                    TripError::Cbor("cbor container shorter than declared".into())
                })?;
                offset += value_len(sub)?;
            }
            Ok(offset)
        }
        // major 7 simple/float：20..=23 单字节，26=float32，27=float64。
        7 => match ai {
            20..=23 => Ok(1),
            26 => Ok(5),
            27 => Ok(9),
            _ => Err(TripError::Cbor(
                "unsupported simple value in deterministic encoding".into(),
            )),
        },
        1 | 3 | 6 => Err(TripError::Cbor(
            "negative int/text/tag not used by TRIP".into(),
        )),
        _ => unreachable!("major is 3 bits, 0..=7"),
    }
}

/// 解析整个输入，拒绝尾部多余字节。
pub fn parse_all(input: &[u8]) -> Result<Value> {
    let (v, rest) = parse_one(input)?;
    if !rest.is_empty() {
        return Err(TripError::Cbor(format!(
            "{} trailing bytes after cbor value",
            rest.len()
        )));
    }
    Ok(v)
}

fn parse_one(input: &[u8]) -> Result<(Value, &[u8])> {
    let (&first, rest) = input
        .split_first()
        .ok_or_else(|| TripError::Cbor("unexpected end of cbor input".into()))?;
    let major = first >> 5;
    let ai = first & 0x1f;

    match major {
        0 => {
            let (n, rest) = read_arg(ai, rest)?;
            Ok((Value::UInt(n), rest))
        }
        1 => Err(TripError::Cbor(
            "negative integers are not used by TRIP".into(),
        )),
        2 => {
            let (len, rest) = read_arg(ai, rest)?;
            let (bytes, rest) = split_len(rest, len as usize, "bstr")?;
            Ok((Value::BStr(bytes.to_vec()), rest))
        }
        3 => Err(TripError::Cbor("text strings are not used by TRIP".into())),
        4 => {
            let (n, rest) = read_arg(ai, rest)?;
            let mut items = Vec::with_capacity(n as usize);
            let mut rest = rest;
            for _ in 0..n {
                let (v, r) = parse_one(rest)?;
                items.push(v);
                rest = r;
            }
            Ok((Value::Array(items), rest))
        }
        5 => {
            let (n, rest) = read_arg(ai, rest)?;
            let mut pairs = Vec::with_capacity(n as usize);
            let mut rest = rest;
            for _ in 0..n {
                let (k, r) = parse_one(rest)?;
                let (val, r) = parse_one(r)?;
                pairs.push((k, val));
                rest = r;
            }
            Ok((Value::Map(pairs), rest))
        }
        6 => Err(TripError::Cbor("cbor tags are not used by TRIP".into())),
        7 => parse_simple(ai, rest),
        _ => unreachable!("major is 3 bits, 0..=7"),
    }
}

fn read_arg(ai: u8, rest: &[u8]) -> Result<(u64, &[u8])> {
    match ai {
        n @ 0..=23 => Ok((n as u64, rest)),
        24 => {
            let (b, rest) = split1(rest)?;
            Ok((b as u64, rest))
        }
        25 => {
            let (b, rest) = split_n(rest, 2)?;
            Ok((u16::from_be_bytes([b[0], b[1]]) as u64, rest))
        }
        26 => {
            let (b, rest) = split_n(rest, 4)?;
            Ok((u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64, rest))
        }
        27 => {
            let (b, rest) = split_n(rest, 8)?;
            let mut arr = [0u8; 8];
            arr.copy_from_slice(b);
            Ok((u64::from_be_bytes(arr), rest))
        }
        28..=30 => Err(TripError::Cbor("reserved cbor additional info".into())),
        31 => Err(TripError::Cbor(
            "indefinite-length encoding is not permitted in TRIP".into(),
        )),
        _ => Err(TripError::Cbor(format!(
            "invalid cbor additional info {ai}"
        ))),
    }
}

fn parse_simple(ai: u8, rest: &[u8]) -> Result<(Value, &[u8])> {
    match ai {
        20 => Ok((Value::Bool(false), rest)),
        21 => Ok((Value::Bool(true), rest)),
        22 => Ok((Value::Null, rest)),
        23 => Err(TripError::Cbor(
            "undefined simple value not supported".into(),
        )),
        24 => Err(TripError::Cbor(
            "1-byte simple values not used by TRIP".into(),
        )),
        25 => {
            // half-float：规范编码禁止；拒绝以避免歧义。
            Err(TripError::Cbor(
                "float16 is not permitted in deterministic TRIP encoding".into(),
            ))
        }
        26 => {
            // float32：读取时提升，但编码器永不产出。
            let (b, rest) = split_n(rest, 4)?;
            let f = f32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64;
            Ok((Value::Float(f), rest))
        }
        27 => {
            let (b, rest) = split_n(rest, 8)?;
            let mut arr = [0u8; 8];
            arr.copy_from_slice(b);
            Ok((Value::Float(f64::from_be_bytes(arr)), rest))
        }
        28..=30 => Err(TripError::Cbor("reserved cbor additional info".into())),
        31 => Err(TripError::Cbor("break stop code not expected here".into())),
        other => Err(TripError::Cbor(format!("unsupported simple value {other}"))),
    }
}

fn split1(rest: &[u8]) -> Result<(u8, &[u8])> {
    rest.split_first()
        .map(|(b, r)| (*b, r))
        .ok_or_else(|| TripError::Cbor("unexpected end while reading argument".into()))
}

fn split_n(rest: &[u8], n: usize) -> Result<(&[u8], &[u8])> {
    split_len(rest, n, "argument")
}

fn split_len<'a>(rest: &'a [u8], len: usize, what: &str) -> Result<(&'a [u8], &'a [u8])> {
    if rest.len() < len {
        return Err(TripError::Cbor(format!(
            "unexpected end while reading {what}: need {len}, have {}",
            rest.len()
        )));
    }
    Ok((&rest[..len], &rest[len..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_stream_of_scalars_and_containers() {
        // uint(5) + bstr(3 字节) + array[uint(1), uint(2)] + map{3: true}
        let mut stream = Vec::new();
        stream.extend_from_slice(Writer::new().uint(5).as_bytes());
        stream.extend_from_slice(Writer::new().bstr(&[1, 2, 3]).as_bytes());
        {
            let mut w = Writer::new();
            w.array(2).uint(1).uint(2);
            stream.extend_from_slice(w.as_bytes());
        }
        {
            let mut w = Writer::new();
            w.map(1).uint(3).bool(true);
            stream.extend_from_slice(w.as_bytes());
        }

        let (v1, rest) = split_value(&stream).unwrap();
        assert_eq!(v1, &[5]);
        let (v2, rest) = split_value(rest).unwrap();
        assert_eq!(v2, &[0x43, 1, 2, 3]);
        let (v3, rest) = split_value(rest).unwrap();
        assert_eq!(v3, &[0x82, 1, 2]);
        let (v4, rest) = split_value(rest).unwrap();
        assert_eq!(v4, &[0xA1, 3, 0xF5]);
        assert!(rest.is_empty());

        // 每个切出的字节都能被完整解析。
        assert!(parse_all(v1).is_ok());
        assert!(parse_all(v2).is_ok());
        assert!(parse_all(v3).is_ok());
        assert!(parse_all(v4).is_ok());
    }

    #[test]
    fn split_truncated_input_errors() {
        // 声明 3 字节 bstr 但只给 1 字节。
        assert!(split_value(&[0x43, 0xAA]).is_err());
        // 空输入。
        assert!(split_value(&[]).is_err());
    }
}
