/** 极简 GPX 解析：`<trkpt>` / `<rtept>` / `<wpt>` 的 lat/lon 与 `<time>`。 */

export interface TrackPoint {
  lat: number;
  lng: number;
  ts: number;
}

/** 解析 GPX 文本为轨迹点（按时间升序）。时间缺失的点会被赋予「合成间隔」。 */
export function parseGpx(
  xml: string,
  opts: { fallbackIntervalSecs: number } = { fallbackIntervalSecs: 900 },
): TrackPoint[] {
  const doc = new DOMParser().parseFromString(xml, 'application/xml');
  if (doc.querySelector('parsererror')) throw new Error('GPX 解析失败：XML 格式不合法');

  const nodes = Array.from(doc.querySelectorAll('trkpt, rtept, wpt'));
  const out: TrackPoint[] = [];
  for (const n of nodes) {
    const lat = Number(n.getAttribute('lat'));
    const lng = Number(n.getAttribute('lon'));
    if (!Number.isFinite(lat) || !Number.isFinite(lng)) continue;
    const timeEl = n.querySelector('time');
    const ts = timeEl ? Math.floor(new Date(timeEl.textContent ?? '').getTime() / 1000) : NaN;
    out.push({ lat, lng, ts });
  }
  if (out.length === 0) throw new Error('GPX 中没有找到任何坐标点');

  // 时间缺失：按声明间隔从「十分钟前」倒推补齐（不早于链尾的约束由链规则处理）
  const base = Math.floor(Date.now() / 1000);
  let missing = 0;
  for (let i = 0; i < out.length; i++) {
    if (!Number.isFinite(out[i]!.ts)) {
      missing++;
      out[i]!.ts = base - (out.length - i) * opts.fallbackIntervalSecs;
    }
  }
  out.sort((a, b) => a.ts - b.ts);
  return out;
}
