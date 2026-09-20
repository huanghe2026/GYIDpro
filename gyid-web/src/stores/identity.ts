// 全局身份 store（Solid singleton store，整个 SPA 共享一份）。
//
// 持久化策略（敏感数据不落明文）：
// - localStorage["gyid.accounts"] = StoredAccount[]（只存 wasm 加密 JSON blob，
//   内含随机 salt + nonce + AES-256-GCM ciphertext）
// - localStorage["gyid.activeId"] = 当前选中的账户 id
// - 解锁会话（seedHex）**只在内存**，刷新/关页即失效，必须重新输入 passphrase

import { createStore } from "solid-js/store";
import { loadJson, saveJson } from "../lib/storage";
import { loadWasm } from "../lib/wasm";

const ACCOUNTS_KEY = "gyid.accounts";
const ACTIVE_KEY = "gyid.activeId";

/** localStorage 中一条加密账户记录。 */
export interface StoredAccount {
  /** 本地 uuid（与公钥无关） */
  id: string;
  /** 用户可读标签 */
  label: string;
  /** wasm encrypt_identity 的输出：{v,label,salt,nonce,ciphertext} JSON 字符串 */
  encJson: string;
  createdAt: number;
}

/** 内存中的解锁会话；刷新页面即清空。 */
export interface UnlockedSession {
  id: string;
  /** Ed25519 seed hex —— 仅存内存，绝不写 localStorage */
  seedHex: string;
  /** Ed25519 公钥 hex（= attester id） */
  pubkeyHex: string;
}

interface IdentityState {
  accounts: StoredAccount[];
  activeId: string | null;
  session: UnlockedSession | null;
  busy: boolean;
  error: string | null;
}

const [state, setState] = createStore<IdentityState>({
  accounts: loadJson<StoredAccount[]>(ACCOUNTS_KEY, []),
  activeId: loadJson<string | null>(ACTIVE_KEY, null),
  session: null,
  busy: false,
  error: null,
});

function persistAccounts(): void {
  saveJson(ACCOUNTS_KEY, state.accounts);
}

/** store 状态（只读使用；变更走 actions） */
export const identityStore = state;

/** 当前选中的账户（可能尚未解锁）。 */
export function activeAccount(): StoredAccount | undefined {
  return state.accounts.find((a) => a.id === state.activeId);
}

/**
 * 新建身份：wasm OsRng 生成 Ed25519 keypair → passphrase 加密 seed →
 * 落 localStorage，并直接进入解锁会话（passphrase 刚输入，无需再解一次）。
 */
export async function createIdentity(
  label: string,
  passphrase: string,
): Promise<UnlockedSession> {
  setState({ busy: true, error: null });
  try {
    const wasm = await loadWasm();
    const kp = wasm.generate_keypair();
    const encJson = wasm.encrypt_identity(kp.seed_hex, passphrase);
    const acc: StoredAccount = {
      id: crypto.randomUUID(),
      label: label.trim() || "default",
      encJson,
      createdAt: Date.now(),
    };
    setState("accounts", (list) => [...list, acc]);
    persistAccounts();
    const session: UnlockedSession = {
      id: acc.id,
      seedHex: kp.seed_hex,
      pubkeyHex: kp.pubkey_hex,
    };
    setState({ activeId: acc.id, session });
    saveJson(ACTIVE_KEY, acc.id);
    return session;
  } catch (e) {
    setState({ error: `创建身份失败：${String(e)}` });
    throw e;
  } finally {
    setState({ busy: false });
  }
}

/** 用 passphrase 解锁指定账户；错误 passphrase 会被 wasm 拒绝（AES-GCM tag 校验）。 */
export async function unlock(
  id: string,
  passphrase: string,
): Promise<UnlockedSession> {
  const acc = state.accounts.find((a) => a.id === id);
  if (!acc) throw new Error("账户不存在");
  setState({ busy: true, error: null });
  try {
    const wasm = await loadWasm();
    const kp = wasm.decrypt_identity(acc.encJson, passphrase);
    const session: UnlockedSession = {
      id,
      seedHex: kp.seed_hex,
      pubkeyHex: kp.pubkey_hex,
    };
    setState({ activeId: id, session });
    saveJson(ACTIVE_KEY, id);
    return session;
  } catch (e) {
    setState({ error: "解锁失败：passphrase 错误或加密数据损坏" });
    throw e;
  } finally {
    setState({ busy: false });
  }
}

/** 切换当前选中的账户（切到别的账户即视为锁定，需要重新输 passphrase）。 */
export function selectAccount(id: string): void {
  if (!state.accounts.some((a) => a.id === id)) return;
  const session = state.session?.id === id ? state.session : null;
  setState({ activeId: id, session });
  saveJson(ACTIVE_KEY, id);
}

/** 锁定：清空内存会话（加密记录保留在 localStorage）。 */
export function lock(): void {
  setState({ session: null });
}

/** 删除账户记录（加密 blob）。注意：该账户的本地面包屑链按 pubkey 存储，需另行清除。 */
export function removeAccount(id: string): void {
  setState("accounts", (list) => list.filter((a) => a.id !== id));
  persistAccounts();
  if (state.activeId === id) {
    const next = state.accounts[0] ?? null;
    setState({ activeId: next?.id ?? null, session: null });
    saveJson(ACTIVE_KEY, next?.id ?? null);
  }
}

export function clearIdentityError(): void {
  setState({ error: null });
}
