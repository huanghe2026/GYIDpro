// 轨迹文件解析器：GPX（XML）与 JPEG EXIF（GPS + 拍摄时间）。
//
// 职责边界（draft-04 §4 / GYIP-0003 §5.5 diliy 改造点 2）：
// - 本模块只做「文件 → 候选点 (lat, lng, ts)」，纯前端、零依赖、无 WASM；
// - H3 量化与 Ed25519 签名仍由 gyid-wasm 完成（见 stores/chain.ts 的 importCrumbs）；
// - 原始 GPS 永不上传 Verifier：仅本地内存 + localStorage 供地图展示。
//
// 支持的时间来源（按可信度降序）：
// 1. GPX <time>（ISO 8601，推荐 UTC）→ 直接可用；
// 2. EXIF GPSDateStamp + GPSTimeStamp（UTC）；
// 3. EXIF DateTimeOriginal（**无时区**，按 UTC 解释并在 UI 提示）；
// 4. 缺时间时按可配置间隔合成（默认 900s，§4.1 默认采集间隔）。

/** 一条候选轨迹点。 */
export interface TrackPoint {
  lat: number;
  lng: number;
  /** Unix 秒 */
  ts: number;
}

/** GPX 解析结果。 */
export interface GpxParseResult {
  points: TrackPoint[];
  /** lat/lon 非法（缺失/越界）而丢弃的点数 */
  invalid: number;
  /** 缺 <time>、由合成时间补齐的点数 */
  synthesized: number;
}

/** 照片解析结果。 */
export interface PhotoPoint extends TrackPoint {
  /** 来源文件名 */
  name: string;
  /** 时间来源：exif = EXIF 内记录；file = 回退到文件修改时间 */
  tsSource: "exif" | "file";
}

export interface PhotoParseResult {
  points: PhotoPoint[];
  failed: { name: string; reason: string }[];
}

export interface ParseOptions {
  /** 缺时间点时合成的时间间隔（秒），下限 300（§4.2 硬下限），默认 900 */
  syntheticIntervalSec?: number;
  /** 合成时间的锚点（Unix 秒）；默认取文件中最早的真实时间，再退化为当前时间 */
  anchorTs?: number;
}

// JPEG 头部最多读 1 MiB：EXIF APP1 必然位于扫描数据（SOS, 0xFFDA）之前。
const JPEG_HEAD_LIMIT = 1 << 20;

// ─────────────────────────────── GPX ───────────────────────────────

/**
 * 解析 GPX 文本，取出 `<trkpt>` / `<rtept>` / `<wpt>` 候选点。
 *
 * - 坐标越界或缺失的节点直接丢弃并计数；
 * - 缺 `<time>` 的节点按文档顺序合成时间（锚点 + 递增 interval）；
 * - 结果按 ts 升序排序。
 */
export function parseGpx(
  xmlText: string,
  opts: ParseOptions = {},
): GpxParseResult {
  const interval = Math.max(
    300,
    Math.floor(opts.syntheticIntervalSec ?? 900),
  );
  const doc = new DOMParser().parseFromString(xmlText, "application/xml");
  const parseErr = doc.querySelector("parsererror");
  if (parseErr) {
    throw new Error(
      `GPX 解析失败：${(parseErr.textContent ?? "XML 格式错误").trim().slice(0, 120)}`,
    );
  }

  const nodes = Array.from(doc.querySelectorAll("trkpt, rtept, wpt"));
  if (nodes.length === 0) {
    throw new Error("未在文件中找到 trkpt / rtept / wpt 轨迹点");
  }

  interface Raw {
    lat: number;
    lng: number;
    ts: number | null;
  }
  const raw: Raw[] = [];
  let invalid = 0;

  for (const node of nodes) {
    const lat = Number(node.getAttribute("lat"));
    const lng = Number(node.getAttribute("lon"));
    if (
      !Number.isFinite(lat) ||
      !Number.isFinite(lng) ||
      Math.abs(lat) > 90 ||
      Math.abs(lng) > 180
    ) {
      invalid++;
      continue;
    }
    let ts: number | null = null;
    const timeText = node.querySelector("time")?.textContent?.trim();
    if (timeText) {
      const ms = Date.parse(timeText);
      if (Number.isFinite(ms)) ts = Math.floor(ms / 1000);
    }
    raw.push({ lat, lng, ts });
  }

  if (raw.length === 0) {
    throw new Error("轨迹点均缺少合法的 lat/lon");
  }

  const known = raw
    .map((r) => r.ts)
    .filter((t): t is number => t !== null);
  let cursor =
    known.length > 0
      ? Math.min(...known)
      : (opts.anchorTs ?? Math.floor(Date.now() / 1000));
  let synthesized = 0;
  const points: TrackPoint[] = [];

  for (const r of raw) {
    if (r.ts !== null) {
      // 保持单调：合成点可能已推进到真实点之后
      cursor = Math.max(cursor, r.ts);
      points.push({ lat: r.lat, lng: r.lng, ts: r.ts });
      continue;
    }
    cursor += interval;
    synthesized++;
    points.push({ lat: r.lat, lng: r.lng, ts: cursor });
  }

  points.sort((a, b) => a.ts - b.ts);
  return { points, invalid, synthesized };
}

// ─────────────────────────── JPEG EXIF ───────────────────────────

interface TiffEntry {
  tag: number;
  type: number;
  count: number;
  /** 12 字节条目的最后 4 字节：值内联或指向值的偏移 */
  valueField: number;
}

/**
 * 从 JPEG 头部提取 GPS 坐标与拍摄时间；无 GPS 则返回 `null`。
 * 非 JPEG（如 HEIC/PNG）同样返回 `null`。
 */
export async function readJpegGps(
  file: File,
): Promise<{ lat: number; lng: number; ts: number | null } | null> {
  const head = new Uint8Array(
    await file.slice(0, JPEG_HEAD_LIMIT).arrayBuffer(),
  );
  if (head.length < 4 || head[0] !== 0xff || head[1] !== 0xd8) return null;

  let i = 2;
  while (i + 4 <= head.length) {
    if (head[i] !== 0xff) return null; // 段结构损坏
    const marker = head[i + 1];
    // 填充字节：标记前允许任意个 0xFF
    if (marker === 0xff) {
      i += 1;
      continue;
    }
    // 无长度字段的独立标记
    if (marker === 0x01 || (marker >= 0xd0 && marker <= 0xd8)) {
      i += 2;
      continue;
    }
    if (marker === 0xda) return null; // 进入扫描数据，EXIF 不会再出现
    const len = (head[i + 2] << 8) | head[i + 3];
    if (len < 2) return null;
    const payload = i + 4;
    const end = i + 2 + len;
    if (end > head.length) return null;

    if (
      marker === 0xe1 &&
      payload + 6 <= head.length &&
      head[payload] === 0x45 && // 'E'
      head[payload + 1] === 0x78 && // 'x'
      head[payload + 2] === 0x69 && // 'i'
      head[payload + 3] === 0x66 && // 'f'
      head[payload + 4] === 0x00 &&
      head[payload + 5] === 0x00
    ) {
      return parseTiffGps(head, payload + 6);
    }
    i = end;
  }
  return null;
}

/** 解析 TIFF/EXIF 结构（`tiffStart` 指向字节序标记）。 */
function parseTiffGps(
  bytes: Uint8Array,
  tiffStart: number,
): { lat: number; lng: number; ts: number | null } | null {
  if (tiffStart + 8 > bytes.length) return null;
  const little =
    bytes[tiffStart] === 0x49 && bytes[tiffStart + 1] === 0x49;
  const big = bytes[tiffStart] === 0x4d && bytes[tiffStart + 1] === 0x4d;
  if (!little && !big) return null;

  const u16 = (o: number): number =>
    little ? bytes[o] | (bytes[o + 1] << 8) : (bytes[o] << 8) | bytes[o + 1];
  const u32 = (o: number): number =>
    little
      ? (bytes[o] |
          (bytes[o + 1] << 8) |
          (bytes[o + 2] << 16) |
          (bytes[o + 3] << 24)) >>>
        0
      : ((bytes[o] << 24) |
          (bytes[o + 1] << 16) |
          (bytes[o + 2] << 8) |
          bytes[o + 3]) >>>
        0;

  if (u16(tiffStart + 2) !== 0x002a) return null; // TIFF magic

  const typeSize = (t: number): number =>
    ({ 1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8 })[t] ?? 0;

  const readIfd = (offAbs: number): TiffEntry[] => {
    if (offAbs + 2 > bytes.length) return [];
    const n = u16(offAbs);
    const out: TiffEntry[] = [];
    for (let k = 0; k < n; k++) {
      const e = offAbs + 2 + k * 12;
      if (e + 12 > bytes.length) break;
      out.push({
        tag: u16(e),
        type: u16(e + 2),
        count: u32(e + 4),
        valueField: e + 8,
      });
    }
    return out;
  };

  // 值的字节长度 ≤ 4 时内联在条目里，否则 valueField 是相对 TIFF 头的偏移
  const dataOffset = (e: TiffEntry): number => {
    const size = typeSize(e.type) * e.count;
    return size <= 4 ? e.valueField : tiffStart + u32(e.valueField);
  };
  const ascii = (e: TiffEntry): string => {
    const o = dataOffset(e);
    let s = "";
    for (let k = 0; k < e.count && o + k < bytes.length; k++) {
      const c = bytes[o + k];
      if (c === 0) break;
      s += String.fromCharCode(c);
    }
    return s.trim();
  };
  const rationals = (e: TiffEntry): number[] => {
    const o = dataOffset(e);
    const out: number[] = [];
    for (let k = 0; k < e.count; k++) {
      const p = o + k * 8;
      if (p + 8 > bytes.length) break;
      const num = u32(p);
      const den = u32(p + 4);
      out.push(den === 0 ? 0 : num / den);
    }
    return out;
  };
  const find = (list: TiffEntry[], tag: number): TiffEntry | undefined =>
    list.find((e) => e.tag === tag);

  const ifd0 = readIfd(tiffStart + u32(tiffStart + 4));

  // GPS IFD（tag 0x8825 指针，type=LONG，值内联）
  let lat: number | null = null;
  let lng: number | null = null;
  let ts: number | null = null;

  const gpsPtr = find(ifd0, 0x8825);
  if (gpsPtr) {
    const gps = readIfd(tiffStart + u32(gpsPtr.valueField));

    const latRefEl = find(gps, 1);
    const latEl = find(gps, 2);
    const lngRefEl = find(gps, 3);
    const lngEl = find(gps, 4);
    if (latEl && lngEl) {
      const la = rationals(latEl);
      const ln = rationals(lngEl);
      if (la.length >= 2 && ln.length >= 2) {
        let laDeg = la[0] + (la[1] ?? 0) / 60 + (la[2] ?? 0) / 3600;
        let lnDeg = ln[0] + (ln[1] ?? 0) / 60 + (ln[2] ?? 0) / 3600;
        const latRef = latRefEl ? ascii(latRefEl).toUpperCase() : "N";
        const lngRef = lngRefEl ? ascii(lngRefEl).toUpperCase() : "E";
        if (latRef.startsWith("S")) laDeg = -laDeg;
        if (lngRef.startsWith("W")) lnDeg = -lnDeg;
        if (Math.abs(laDeg) <= 90 && Math.abs(lnDeg) <= 180) {
          lat = laDeg;
          lng = lnDeg;
        }
      }
    }

    // GPSDateStamp + GPSTimeStamp 均为 UTC
    const dateEl = find(gps, 29);
    const timeEl = find(gps, 7);
    if (dateEl && timeEl) {
      const m = /^(\d{4}):(\d{2}):(\d{2})/.exec(ascii(dateEl));
      const t = rationals(timeEl);
      if (m && t.length >= 3) {
        ts = Math.floor(
          Date.UTC(
            Number(m[1]),
            Number(m[2]) - 1,
            Number(m[3]),
            Math.floor(t[0]),
            Math.floor(t[1]),
            Math.floor(t[2]),
          ) / 1000,
        );
      }
    }
  }

  if (lat === null || lng === null) return null;

  // 回退：DateTimeOriginal / DateTimeDigitized（无时区，按 UTC 解释）
  if (ts === null) {
    const exifPtr = find(ifd0, 0x8769);
    if (exifPtr) {
      const exifIfd = readIfd(tiffStart + u32(exifPtr.valueField));
      const dto = find(exifIfd, 0x9003) ?? find(exifIfd, 0x9004);
      if (dto) {
        const m =
          /^(\d{4}):(\d{2}):(\d{2})[ T](\d{2}):(\d{2}):(\d{2})/.exec(
            ascii(dto),
          );
        if (m) {
          ts = Math.floor(
            Date.UTC(
              Number(m[1]),
              Number(m[2]) - 1,
              Number(m[3]),
              Number(m[4]),
              Number(m[5]),
              Number(m[6]),
            ) / 1000,
          );
        }
      }
    }
  }

  return { lat, lng, ts };
}

/**
 * 批量解析照片（JPEG）。无 GPS 的返回在 `failed` 中，不打断其余照片。
 */
export async function parsePhotoFiles(
  files: File[],
): Promise<PhotoParseResult> {
  const points: PhotoPoint[] = [];
  const failed: { name: string; reason: string }[] = [];

  for (const file of files) {
    try {
      const gps = await readJpegGps(file);
      if (!gps) {
        failed.push({ name: file.name, reason: "无 GPS EXIF（或非 JPEG）" });
        continue;
      }
      points.push({
        lat: gps.lat,
        lng: gps.lng,
        ts: gps.ts ?? Math.floor(file.lastModified / 1000),
        name: file.name,
        tsSource: gps.ts === null ? "file" : "exif",
      });
    } catch (e) {
      failed.push({
        name: file.name,
        reason: e instanceof Error ? e.message : String(e),
      });
    }
  }

  points.sort((a, b) => a.ts - b.ts);
  return { points, failed };
}
