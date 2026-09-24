/**
 * 验证方客户端（同源）。
 *
 * 全部走相对路径：`/v1/*` 与 `wss://<host>/v1/challenge` 由 nginx 反代到本机
 * `trip-server`（127.0.0.1:8080）。因为同源，无需 CORS，也不存在混合内容问题。
 */

const API = ''; // 同源：留空即相对路径

export interface IdentityInfo {
  attester: string;
  breadcrumb_count: number;
  unique_cells: number;
  chain_head: string;
  last_ts: number;
}

export interface VerifierWellKnown {
  verifier: string;
  verifier_pubkey: string;
  protocol: string;
  validity_secs: number;
  challenge_ttl_secs: number;
  policy: { alpha_bio_range: [number, number]; min_confidence: number; min_trust: number };
}

export interface ChallengeInfo {
  challenge_id: string;
  expires_at: number;
  delivered: boolean;
}

export type PohFetch = { kind: 'issued'; bytes: Uint8Array } | { kind: 'pending'; expiresAt: number };

async function toError(res: Response, what: string): Promise<Error> {
  let detail = '';
  try {
    detail = await res.text();
  } catch {
    /* ignore */
  }
  return new Error(`${what}: HTTP ${res.status}${detail ? ` — ${detail.slice(0, 300)}` : ''}`);
}

/** 验证方公钥与策略。 */
export async function fetchWellKnown(): Promise<VerifierWellKnown> {
  const res = await fetch(`${API}/.well-known/verifier.json`, { cache: 'no-store' });
  if (!res.ok) throw await toError(res, 'verifier.json');
  return (await res.json()) as VerifierWellKnown;
}

/** 某 attester 的链状态；404 表示该身份尚无证据。 */
export async function fetchIdentity(pubkeyHex: string): Promise<IdentityInfo | null> {
  const res = await fetch(`${API}/v1/identity/${pubkeyHex}`, { cache: 'no-store' });
  if (res.status === 404) return null;
  if (!res.ok) throw await toError(res, 'identity');
  return (await res.json()) as IdentityInfo;
}

/** 上传一批面包屑（CBOR 帧流 hex）。 */
export async function uploadEvidence(cborHex: string): Promise<{ stored: number; unique_cells: number; chain_head: string }> {
  // 复制到一个独立的 ArrayBuffer：既满足 BodyInit/BlobPart 的类型约束，
  // 也避免把 WASM 内存视图直接交给 fetch（可转移缓冲可能被 detach）。
  const src = hexToBytes(cborHex);
  const ab = new ArrayBuffer(src.length);
  new Uint8Array(ab).set(src);

  const res = await fetch(`${API}/v1/evidence`, {
    method: 'POST',
    headers: { 'content-type': 'application/octet-stream' },
    body: ab,
  });
  if (!res.ok) throw await toError(res, 'evidence');
  return (await res.json()) as { stored: number; unique_cells: number; chain_head: string };
}

/** 发起 Active Verification：创建绑定 RP nonce 的挑战。 */
export async function requestChallenge(attesterHex: string, rpNonceHex: string, ttlSecs = 90): Promise<ChallengeInfo> {
  const res = await fetch(`${API}/v1/verify`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ attester: attesterHex, rp_nonce: rpNonceHex, ttl_secs: ttlSecs }),
  });
  if (!res.ok) throw await toError(res, 'verify');
  return (await res.json()) as ChallengeInfo;
}

/** 轮询取回 PoH 证书。 */
export async function fetchPoh(challengeId: string): Promise<PohFetch> {
  const res = await fetch(`${API}/v1/poh/${challengeId}`, { cache: 'no-store' });
  if (res.status === 202) {
    const j = (await res.json()) as { expires_at?: number };
    return { kind: 'pending', expiresAt: j.expires_at ?? 0 };
  }
  if (!res.ok) throw await toError(res, 'poh');
  return { kind: 'issued', bytes: new Uint8Array(await res.arrayBuffer()) };
}

/**
 * 打开 Attester 侧实时通道，收到挑战即用身份密钥签名回送。
 * 返回一个可关闭的句柄；`onEvent` 用于 UI 日志。
 */
export function openChallengeChannel(
  pubkeyHex: string,
  sign: (challengeCborHex: string) => string,
  onEvent: (kind: 'ready' | 'challenge' | 'sent' | 'ack' | 'error' | 'closed', detail?: unknown) => void,
): { close: () => void } {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const ws = new WebSocket(`${proto}//${location.host}/v1/challenge?attester=${pubkeyHex}`);
  ws.binaryType = 'arraybuffer';

  ws.onopen = () => {
    // 身份注册帧：让服务端把该 attester 的挑战路由到本连接
    ws.send(JSON.stringify({ type: 'hello', attester: pubkeyHex }));
    onEvent('ready');
  };

  ws.onmessage = (ev) => {
    try {
      if (typeof ev.data === 'string') {
        const msg = JSON.parse(ev.data) as { type?: string; error?: string; ok?: boolean };
        if (msg.error) onEvent('error', msg.error);
        else onEvent('ack', msg);
        return;
      }
      // 二进制帧 = LivenessChallenge 的 CBOR
      const bytes = new Uint8Array(ev.data as ArrayBuffer);
      const cborHex = bytesToHex(bytes);
      onEvent('challenge', cborHex);
      const respHex = sign(cborHex);
      ws.send(hexToBytes(respHex));
      onEvent('sent');
    } catch (e) {
      onEvent('error', e);
    }
  };

  ws.onerror = (e) => onEvent('error', e);
  ws.onclose = () => onEvent('closed');

  return { close: () => ws.close() };
}

// ── hex 助手 ──────────────────────────────────────────────────

export function bytesToHex(bytes: Uint8Array): string {
  let s = '';
  for (const b of bytes) s += b.toString(16).padStart(2, '0');
  return s;
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  return out;
}

/** 生成 16 字节随机 nonce（RP 侧）。 */
export function randomNonceHex(bytes = 16): string {
  const buf = new Uint8Array(bytes);
  crypto.getRandomValues(buf);
  return bytesToHex(buf);
}
