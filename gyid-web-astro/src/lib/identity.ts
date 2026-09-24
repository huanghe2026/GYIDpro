/**
 * 身份与会话（框架无关的单例）。
 *
 * 持久化策略（敏感数据不落明文）：
 * - localStorage["gyid.accounts"]  = StoredAccount[]（仅存 wasm 加密 blob：
 *   随机 salt + nonce + AES-256-GCM 密文）
 * - localStorage["gyid.activeId"]  = 当前账户 id
 * - 解锁会话（seedHex）**只在模块内存**，锁页/刷新即失效。
 *
 * 与 gyid-web 使用同名 key，两个控制台可共享同一批账户。
 */
import { loadWasm, type KeypairJs } from './wasm';
import { loadJson, saveJson } from './storage';

const ACCOUNTS_KEY = 'gyid.accounts';
const ACTIVE_KEY = 'gyid.activeId';

export interface StoredAccount {
  id: string;
  label: string;
  /** wasm encrypt_identity 输出的 JSON 字符串 */
  encJson: string;
  createdAt: number;
}

export interface UnlockedSession {
  id: string;
  /** Ed25519 seed hex —— 仅内存 */
  seedHex: string;
  /** Ed25519 公钥 hex（= attester id） */
  pubkeyHex: string;
}

interface IdentityState {
  accounts: StoredAccount[];
  activeId: string | null;
  session: UnlockedSession | null;
}

const state: IdentityState = {
  accounts: loadJson<StoredAccount[]>(ACCOUNTS_KEY, []),
  activeId: loadJson<string | null>(ACTIVE_KEY, null),
  session: null,
};

/** 订阅者（页面 UI 在状态变化后重绘）。 */
type Listener = () => void;
const listeners = new Set<Listener>();

export function subscribe(fn: Listener): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

function emit(): void {
  for (const fn of listeners) fn();
}

export function getState(): Readonly<IdentityState> {
  return state;
}

export function activeAccount(): StoredAccount | undefined {
  return state.accounts.find((a) => a.id === state.activeId);
}

function persist(): void {
  saveJson(ACCOUNTS_KEY, state.accounts);
}

/** 新建身份：生成密钥 → 口令加密 → 落盘 → 直接进入解锁态。 */
export async function createIdentity(label: string, passphrase: string): Promise<UnlockedSession> {
  if (passphrase.length < 6) throw new Error('passphrase must be at least 6 characters');
  const wasm = await loadWasm();
  const kp: KeypairJs = wasm.generate_keypair();
  const encJson = wasm.encrypt_identity(kp.seed_hex, passphrase);
  const acc: StoredAccount = {
    id: crypto.randomUUID(),
    label: label.trim() || 'default',
    encJson,
    createdAt: Date.now(),
  };
  state.accounts = [...state.accounts, acc];
  persist();
  const session: UnlockedSession = { id: acc.id, seedHex: kp.seed_hex, pubkeyHex: kp.pubkey_hex };
  state.activeId = acc.id;
  state.session = session;
  saveJson(ACTIVE_KEY, acc.id);
  emit();
  return session;
}

/** 用口令解锁；错误口令会被 AES-GCM tag 校验拒绝。 */
export async function unlock(id: string, passphrase: string): Promise<UnlockedSession> {
  const acc = state.accounts.find((a) => a.id === id);
  if (!acc) throw new Error('account not found');
  const wasm = await loadWasm();
  const kp = wasm.decrypt_identity(acc.encJson, passphrase);
  const session: UnlockedSession = { id, seedHex: kp.seed_hex, pubkeyHex: kp.pubkey_hex };
  state.activeId = id;
  state.session = session;
  saveJson(ACTIVE_KEY, id);
  emit();
  return session;
}

/** 切换账户（切走即视为锁定，需重新输口令）。 */
export function selectAccount(id: string): void {
  if (!state.accounts.some((a) => a.id === id)) return;
  state.activeId = id;
  state.session = state.session?.id === id ? state.session : null;
  saveJson(ACTIVE_KEY, id);
  emit();
}

/** 锁定：只清内存会话，加密记录保留。 */
export function lock(): void {
  state.session = null;
  emit();
}

/** 删除账户（加密 blob）。本地链按 pubkey 单独存储，需另行清理。 */
export function removeAccount(id: string): void {
  state.accounts = state.accounts.filter((a) => a.id !== id);
  persist();
  if (state.activeId === id) {
    state.activeId = state.accounts[0]?.id ?? null;
    state.session = null;
    saveJson(ACTIVE_KEY, state.activeId);
  }
  emit();
}
