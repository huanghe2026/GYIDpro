// localStorage 类型安全包装。所有 GyID 数据统一 `gyid.*` 前缀。

/** 读取并 JSON.parse；不存在 / 损坏时返回 fallback（不抛异常）。 */
export function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

/** JSON.stringify 后写入。 */
export function saveJson(key: string, value: unknown): void {
  localStorage.setItem(key, JSON.stringify(value));
}

/** 删除一个 key（不存在也不报错）。 */
export function removeKey(key: string): void {
  localStorage.removeItem(key);
}
