/* tslint:disable */
/* eslint-disable */
/**
 * Attester 回送的活体响应（draft-04 §12）。
 */
export interface LivenessResponseJs {
    challenge_id_hex: string;
    rp_nonce_hex: string;
    chain_head_hex: string;
    index: number;
    attester_signature_hex: string;
}

/**
 * PoH 证书解析结果（RP 侧）。
 */
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
    /**
     * 签名有效 + 未过期 + nonce 匹配。
     */
    fresh: boolean;
    /**
     * α / 置信度 / 信任分达到门槛。
     */
    policy_pass: boolean;
    /**
     * fresh && policy_pass。
     */
    is_trusted: boolean;
}

/**
 * Verifier 下发的活体挑战（draft-04 §12）。
 */
export interface LivenessChallengeJs {
    challenge_id_hex: string;
    rp_nonce_hex: string;
    chain_head_hex: string;
    expected_index: number;
    deadline: number;
}

/**
 * 一条已签名面包屑（draft-04 §3）。所有字节字段都是 hex 字符串。
 */
export interface BreadcrumbJs {
    /**
     * 链内序号（从 0 起连续）。
     */
    index: number;
    /**
     * Attester 公钥 hex。
     */
    pubkey_hex: string;
    /**
     * Unix 秒时间戳。
     */
    timestamp: number;
    /**
     * H3 cell 的 hex（16 字符）。**不用 number**：res 7..=10 的 cell 值
     * 约 6×10^17，超过 JS Number.MAX_SAFE_INTEGER（2^53），会丢精度。
     */
    h3_cell_hex: string;
    /**
     * H3 分辨率（7..=10）。
     */
    h3_resolution: number;
    /**
     * §2.2 context digest hex（32 字节）。
     */
    context_digest_hex: string;
    /**
     * 上一条面包屑的 block_hash hex；创世为 null。
     */
    prev_hash_hex: string | undefined;
    /**
     * 当前块哈希 hex（= SHA-256(to_cbor)）。
     */
    block_hash_hex: string;
    /**
     * §4.2 探索会话标志。
     */
    exploration: boolean;
    /**
     * Wi-Fi 分量是否在 context digest 中。
     */
    wifi_present: boolean;
    /**
     * 基站分量是否在 context digest 中。
     */
    cell_present: boolean;
    /**
     * IMU 分量是否在 context digest 中。
     */
    imu_present: boolean;
    /**
     * Attester Ed25519 签名 hex（64 字节）。
     */
    signature_hex: string;
}

/**
 * 身份密钥对（seed 用于本地加密存储，pubkey 公开上链）。
 */
export interface KeypairJs {
    /**
     * 32 字节 Ed25519 seed 的 hex（64 字符）。**仅供本地存储/调试，不要外发**。
     */
    seed_hex: string;
    /**
     * 32 字节 Ed25519 公钥的 hex（64 字符）。
     */
    pubkey_hex: string;
}


/**
 * 启动钩子（可选）。前端 `initWasm()` 调用一次以设置 panic hook。
 */
export function _start(): void;

/**
 * CBOR hex → 单条面包屑。
 */
export function breadcrumb_from_cbor_hex(cbor_hex: string): BreadcrumbJs;

/**
 * 单条面包屑 → CBOR hex（用于调试或自定义拼接）。
 */
export function breadcrumb_to_cbor_hex(crumb: BreadcrumbJs): string;

/**
 * 从 CBOR 帧流 hex 解析出多条面包屑（`GET /v1/evidence` 响应或本地存储读回）。
 */
export function chain_from_cbor_hex(cbor_hex: string): BreadcrumbJs[];

/**
 * 把多条面包屑拼成 CBOR 帧流 hex，直接作为 `POST /v1/evidence` body。
 *
 * 入参可以是**任意连续切片**（续传时只传后缀，如 [#100..#399]）：
 * 服务端按 index 二分合并，不要求批次从创世 #0 开始。因此这里只做序列化，
 * 不用 [`Chain::append`]（那个要求本批自 index=0 起，会拒掉合法后缀批次）。
 */
export function chain_to_cbor_hex(crumbs: BreadcrumbJs[]): string;

/**
 * 采集一条面包屑并签名。`prev_index_opt` / `prev_block_hash_hex_opt` 为 `None`
 * 表示创世（第一条）；否则取链尾的 index + block_hash。
 *
 * 返回的 `BreadcrumbJs.block_hash_hex` 即下一条的 `prev_block_hash_hex`。
 */
export function collect_breadcrumb(seed_hex: string, lat: number, lng: number, h3_resolution: number, timestamp: bigint, prev_index_opt: bigint | null | undefined, prev_block_hash_hex_opt: string | null | undefined, exploration: boolean): BreadcrumbJs;

/**
 * 用 passphrase 解密之前 `encrypt_identity` 输出的 JSON。
 * 成功返回 KeypairJs（含 seed_hex + pubkey_hex）。
 */
export function decrypt_identity(json: string, passphrase: string): KeypairJs;

/**
 * 从 hex seed（64 字符）派生公钥 hex。用于解锁后显示。
 */
export function derive_pubkey_hex(seed_hex: string): string;

/**
 * 用 passphrase 加密 seed，返回可落盘的 JSON 字符串。
 * PBKDF2(SHA-256, 250k iters) → AES-256-GCM。同一 passphrase 每次输出不同。
 */
export function encrypt_identity(seed_hex: string, passphrase: string): string;

/**
 * 生成新的 Ed25519 身份（OsRng 真随机）。
 * 返回 seed_hex（**仅供本地加密存储，不要外发**）+ pubkey_hex。
 */
export function generate_keypair(): KeypairJs;

/**
 * 把 (lat, lng) 量化为 H3 cell（u64），返回 hex 字符串。
 * 协议要求分辨率 7..=10。
 */
export function h3_to_cell_hex(lat: number, lng: number, resolution: number): string;

/**
 * 解析 Verifier 下发的 LivenessChallenge CBOR hex（用于 Verify 页调试显示）。
 */
export function parse_liveness_challenge(cbor_hex: string): LivenessChallengeJs;

/**
 * 解析 LivenessResponse CBOR hex（调试用）。
 */
export function parse_liveness_response(cbor_hex: string): LivenessResponseJs;

/**
 * Attester 用身份密钥对挑战签名，返回 LivenessResponse CBOR hex。
 * 直接通过实时 WS 通道回送 Verifier。
 */
export function sign_liveness_response(seed_hex: string, challenge_cbor_hex: string): string;

/**
 * 用 `ChainRules::default()` 跑全链自检（§4.1/§4.2/§4.3）。
 * 返回 `true` 表示通过，`false` 表示违规（具体原因看 console error）。
 */
export function verify_chain(crumbs: BreadcrumbJs[]): boolean;

/**
 * 校验 PoH 证书：CBOR 解析 → Verifier 签名 → 新鲜性 → 策略。
 *
 * - `poh_hex`：PoH 证书 CBOR hex（从 `GET /v1/poh` 拿到）；
 * - `verifier_pubkey_hex`：Verifier 公钥 hex（从 `GET /.well-known/verifier.json` 拿到）；
 * - `expected_nonce_hex`：RP 自己生成的 16 字节 nonce hex；
 * - `now_unix`：当前 Unix 秒；
 * - `min_confidence` / `min_trust`：策略门槛（draft-04 §10 §2–4）。
 */
export function verify_poh(poh_hex: string, verifier_pubkey_hex: string, expected_nonce_hex: string, now_unix: bigint, min_confidence: number, min_trust: number): PohInfoJs;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly breadcrumb_from_cbor_hex: (a: number, b: number, c: number) => void;
    readonly breadcrumb_to_cbor_hex: (a: number, b: number) => void;
    readonly chain_from_cbor_hex: (a: number, b: number, c: number) => void;
    readonly chain_to_cbor_hex: (a: number, b: number, c: number) => void;
    readonly collect_breadcrumb: (a: number, b: number, c: number, d: number, e: number, f: number, g: bigint, h: number, i: bigint, j: number, k: number, l: number) => void;
    readonly decrypt_identity: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly derive_pubkey_hex: (a: number, b: number, c: number) => void;
    readonly encrypt_identity: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly generate_keypair: (a: number) => void;
    readonly h3_to_cell_hex: (a: number, b: number, c: number, d: number) => void;
    readonly parse_liveness_challenge: (a: number, b: number, c: number) => void;
    readonly parse_liveness_response: (a: number, b: number, c: number) => void;
    readonly sign_liveness_response: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly verify_chain: (a: number, b: number, c: number) => void;
    readonly verify_poh: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: bigint, i: number, j: number) => void;
    readonly _start: () => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
