// Verifier HTTP/WS 客户端：封装 trip-server 的全部 REST + WS 端点
const VERIFIER_URL =
  import.meta.env.VITE_VERIFIER_URL ?? "http://localhost:8080";

/** GET /v1/identity/:hex 响应体 */
export interface IdentityInfo {
  attester: string;
  breadcrumb_count: number;
  unique_cells: number;
  chain_head: string;
  last_ts: number;
}

/** POST /v1/evidence 响应体（stored = 服务端合并后的面包屑总数） */
export interface UploadResponse {
  identity: string;
  stored: number;
  unique_cells: number;
  chain_head: string;
}

/** POST /v1/verify 响应体 */
export interface ChallengeInfo {
  challenge_id: string;
  expires_at: number;
  delivered: boolean;
}

/** GET /v1/pohs 响应体 */
export interface PohListInfo {
  attester: string;
  challenge_ids: string[];
  count: number;
}

/** GET /v1/explorer 单个身份条目 */
export interface ExplorerIdentity {
  attester: string;
  breadcrumb_count: number;
  unique_cells: number;
  chain_head: string;
  last_ts: number;
  poh_count: number;
}

/** GET /v1/explorer 响应体 */
export interface ExplorerInfo {
  total_identities: number;
  total_breadcrumbs: number;
  identities: ExplorerIdentity[];
}

/** hex 字符串 → Uint8Array */
export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.length % 2 === 1 ? "0" + hex : hex;
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

/** Uint8Array → hex 字符串（小写） */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * GET /v1/identity/:pubkey — 拉取 attester 公开身份信息。
 * 404（从未上传过证据）返回 null，由调用方展示"暂无链上数据"。
 */
export async function fetchIdentity(
  pubkeyHex: string,
): Promise<IdentityInfo | null> {
  const res = await fetch(`${VERIFIER_URL}/v1/identity/${pubkeyHex}`);
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`fetchIdentity: HTTP ${res.status}`);
  return (await res.json()) as IdentityInfo;
}

/** POST /v1/evidence — 上传面包屑 CBOR 帧（octet-stream body），服务端校验后按链合并 */
export async function uploadEvidence(cborHex: string): Promise<UploadResponse> {
  const res = await fetch(`${VERIFIER_URL}/v1/evidence`, {
    method: "POST",
    headers: { "Content-Type": "application/octet-stream" },
    // Uint8Array 兼容运行时 BodyInit，但 TS lib 类型不直接收，故断言
    body: hexToBytes(cborHex) as BodyInit,
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`uploadEvidence: HTTP ${res.status} ${text}`);
  }
  return (await res.json()) as UploadResponse;
}

/** POST /v1/verify — 请求一次 Active Verification 挑战 */
export async function requestChallenge(
  attesterHex: string,
  rpNonceHex: string,
): Promise<ChallengeInfo> {
  const res = await fetch(`${VERIFIER_URL}/v1/verify`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ attester: attesterHex, rp_nonce: rpNonceHex }),
  });
  if (!res.ok) throw new Error(`requestChallenge: HTTP ${res.status}`);
  return (await res.json()) as ChallengeInfo;
}

/**
 * POST /v1/poh — 凭 challenge_id 取 PoH 证书。
 * 200 = application/cbor 二进制；202 = 待应答（继续轮询）；
 * 410 = 已过期/已消费；404 = 未知 challenge。
 */
export async function fetchPoh(
  challengeIdHex: string,
): Promise<Uint8Array | { status: "pending"; expires_at: number }> {
  const res = await fetch(`${VERIFIER_URL}/v1/poh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ challenge_id: challengeIdHex }),
  });
  if (res.status === 202) {
    const j = (await res.json()) as { status: string; expires_at: number };
    return { status: "pending", expires_at: j.expires_at };
  }
  if (!res.ok) throw new Error(`fetchPoh: HTTP ${res.status}`);
  const buf = await res.arrayBuffer();
  return new Uint8Array(buf);
}

/**
 * GET /v1/pohs?attester=... — 列出某 attester 的全部 PoH 证书。
 * 404（从未签发过）返回 null，由调用方展示空态引导。
 */
export async function listPohs(
  attesterHex: string,
): Promise<PohListInfo | null> {
  const res = await fetch(`${VERIFIER_URL}/v1/pohs?attester=${attesterHex}`);
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`listPohs: HTTP ${res.status}`);
  return (await res.json()) as PohListInfo;
}

/**
 * GET /v1/explorer — 全网身份聚合浏览（空 Verifier 返回空数组，不是 404）。
 */
export async function fetchExplorer(): Promise<ExplorerInfo> {
  const res = await fetch(`${VERIFIER_URL}/v1/explorer`);
  if (!res.ok) throw new Error(`fetchExplorer: HTTP ${res.status}`);
  return (await res.json()) as ExplorerInfo;
}

/** GET /.well-known/verifier.json — 拉取 verifier 公钥 hex */
export async function fetchVerifierPubkeyHex(): Promise<string> {
  const res = await fetch(`${VERIFIER_URL}/.well-known/verifier.json`);
  if (!res.ok) throw new Error(`fetchVerifierPubkeyHex: HTTP ${res.status}`);
  const data = (await res.json()) as { verifier_pubkey: string };
  return data.verifier_pubkey;
}

/** 打开 challenge WS 通道（binaryType=arraybuffer，双向收发 CBOR） */
export function openChallengeWs(attesterHex: string): WebSocket {
  const wsUrl = `${VERIFIER_URL.replace(/^http/, "ws")}/v1/challenge?attester=${attesterHex}`;
  const ws = new WebSocket(wsUrl);
  ws.binaryType = "arraybuffer";
  return ws;
}
