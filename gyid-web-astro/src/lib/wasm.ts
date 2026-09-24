/**
 * gyid-wasm 加载器与 JS 侧类型。
 *
 * WASM 由同一份 `trip-core` Rust 代码（经 `gyid-wasm` 的 wasm-bindgen 胶水层）
 * 编译而来，放在 `public/wasm/` 下作为静态资源，运行时按需 `import()`，
 * 避免被 Vite 预打包（胶水层依赖 `import.meta.url` 解析 `.wasm` 文件）。
 */

/** 身份密钥对。seed 仅用于本地加密存储，绝不外发。 */
export interface KeypairJs {
  seed_hex: string;
  pubkey_hex: string;
}

/** 一条已签名面包屑（draft-04 §3）。字节字段一律 hex 字符串。 */
export interface BreadcrumbJs {
  index: number;
  pubkey_hex: string;
  timestamp: number;
  /** H3 cell 的 hex（16 字符）。不用 number：res10 的值超出 JS 安全整数范围。 */
  h3_cell_hex: string;
  h3_resolution: number;
  context_digest_hex: string;
  prev_hash_hex: string | undefined;
  block_hash_hex: string;
  exploration: boolean;
  wifi_present: boolean;
  cell_present: boolean;
  imu_present: boolean;
  signature_hex: string;
}

/** Verifier 下发的活体挑战（draft-04 §12）。 */
export interface LivenessChallengeJs {
  challenge_id_hex: string;
  rp_nonce_hex: string;
  chain_head_hex: string;
  expected_index: number;
  deadline: number;
}

/** PoH 证书解析结果。 */
export interface PohInfoJs {
  identity_hex: string;
  issued_at: number;
  epoch_count: number;
  alpha: number;
  beta: number;
  kappa: number;
  pi: number;
  criticality_confidence: number;
  trust: number;
  unique_cells: number;
  breadcrumb_count: number;
  validity_secs: number;
  nonce_hex: string;
  chain_head_hex: string;
  verifier_signature_hex: string;
  fresh: boolean;
  policy_pass: boolean;
  is_trusted: boolean;
}

export interface GyidWasm {
  generate_keypair(): KeypairJs;
  derive_pubkey_hex(seed_hex: string): string;
  encrypt_identity(seed_hex: string, passphrase: string): string;
  decrypt_identity(json: string, passphrase: string): KeypairJs;
  h3_to_cell_hex(lat: number, lng: number, resolution: number): string;
  collect_breadcrumb(
    seed_hex: string,
    lat: number,
    lng: number,
    h3_resolution: number,
    timestamp: bigint,
    prev_index_opt: bigint | null | undefined,
    prev_block_hash_hex_opt: string | null | undefined,
    exploration: boolean,
  ): BreadcrumbJs;
  breadcrumb_to_cbor_hex(crumb: BreadcrumbJs): string;
  breadcrumb_from_cbor_hex(cbor_hex: string): BreadcrumbJs;
  chain_to_cbor_hex(crumbs: BreadcrumbJs[]): string;
  chain_from_cbor_hex(cbor_hex: string): BreadcrumbJs[];
  verify_chain(crumbs: BreadcrumbJs[]): boolean;
  parse_liveness_challenge(cbor_hex: string): LivenessChallengeJs;
  sign_liveness_response(seed_hex: string, challenge_cbor_hex: string): string;
  verify_poh(
    poh_hex: string,
    verifier_pubkey_hex: string,
    expected_nonce_hex: string,
    now_unix: bigint,
    min_confidence: number,
    min_trust: number,
  ): PohInfoJs;
}

// 跟随部署基路径（根部署 `/wasm/…`；子目录部署 `/verify/wasm/…`）。
const WASM_GLUE_URL = `${import.meta.env.BASE_URL}wasm/gyid_wasm.js`;

let pending: Promise<GyidWasm> | null = null;

/**
 * 载入并初始化 WASM（幂等；跨 Astro View Transitions 导航复用同一实例）。
 */
export function loadWasm(): Promise<GyidWasm> {
  if (!pending) {
    pending = (async () => {
      const mod = (await import(/* @vite-ignore */ WASM_GLUE_URL)) as {
        default: (input?: unknown) => Promise<unknown>;
      } & GyidWasm;
      // 不传参：胶水层按 import.meta.url 解析同目录的 gyid_wasm_bg.wasm
      await mod.default();
      return mod as unknown as GyidWasm;
    })();
  }
  return pending;
}

/** 当前 Unix 秒。 */
export function nowSecs(): number {
  return Math.floor(Date.now() / 1000);
}
