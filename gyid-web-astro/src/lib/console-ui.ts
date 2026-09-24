/**
 * 控制台共享 UI：身份状态条 + 会话事件。
 *
 * 四个控制台页各自是独立 HTML，但 Astro 的 View Transitions 让站内跳转变成
 * 客户端换页，因此 `identity.ts` 的模块状态（含解锁后的 seed）在页间保持，
 * seed 始终只存在于内存。
 *
 * 会话变化通过 `window` 上的 `gyid:session` 自定义事件广播：
 *   detail = { session: UnlockedSession | null }
 */
import {
  activeAccount,
  createIdentity,
  getState,
  lock,
  removeAccount,
  selectAccount,
  subscribe,
  unlock,
  type UnlockedSession,
} from './identity';
import { loadChain } from './chain';
import type { Lang } from '../i18n/ui';

export const SESSION_EVENT = 'gyid:session';

// 控制台页面统一从本模块取身份 API，避免每页重复 import 两个模块
export { activeAccount, getState, lock, removeAccount, selectAccount, subscribe } from './identity';
export type { StoredAccount, UnlockedSession } from './identity';

export interface ConsoleStrings {
  unlocked: string;
  locked: string;
  lock: string;
  unlock: string;
  newIdentity: string;
  cancel: string;
  label: string;
  passphrase: string;
  confirmPassphrase: string;
  create: string;
  selectAccount: string;
  remove: string;
  copy: string;
  copied: string;
  accounts: string;
  passphraseMismatch: string;
  passphraseTooShort: string;
  unlockFailed: string;
}

const enStrings: ConsoleStrings = {
  unlocked: 'Unlocked (key in memory only)',
  locked: 'Locked',
  lock: 'Lock',
  unlock: 'Unlock',
  newIdentity: '+ New identity',
  cancel: 'Cancel',
  label: 'Label (optional)',
  passphrase: 'Passphrase (min 6 chars)',
  confirmPassphrase: 'Repeat passphrase',
  create: 'Generate and save',
  selectAccount: 'Account',
  remove: 'Remove',
  copy: 'Copy',
  copied: 'Copied',
  accounts: 'accounts',
  passphraseMismatch: 'Passphrases do not match',
  passphraseTooShort: 'Passphrase must be at least 6 characters',
  unlockFailed: 'Could not unlock — wrong passphrase?',
};

const zhStrings: ConsoleStrings = {
  unlocked: '已解锁（密钥仅在内存）',
  locked: '已锁定',
  lock: '锁定',
  unlock: '解锁',
  newIdentity: '+ 新建身份',
  cancel: '取消',
  label: '标签（可选）',
  passphrase: '口令（至少 6 位）',
  confirmPassphrase: '再输一遍口令',
  create: '生成并加密保存',
  selectAccount: '账户',
  remove: '删除',
  copy: '复制',
  copied: '已复制',
  accounts: '账户',
  passphraseMismatch: '两次输入的口令不一致',
  passphraseTooShort: '口令至少 6 位',
  unlockFailed: '解锁失败——口令是否正确？',
};

export function stringsFor(lang: Lang): ConsoleStrings {
  return lang === 'zh' ? zhStrings : enStrings;
}

const esc = (s: string): string =>
  s.replace(/[&<>"']/g, (c) =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c] as string,
  );

const short = (hex: string): string => (hex.length > 20 ? `${hex.slice(0, 10)}…${hex.slice(-8)}` : hex);

/** 当前会话（未解锁为 null）。 */
export function currentSession(): UnlockedSession | null {
  return getState().session;
}

/**
 * 挂载身份状态条。返回清理函数（View Transitions 换页时由 astro:before-swap 调用）。
 */
export function mountIdentityBar(lang: Lang): () => void {
  const host = document.getElementById('gyid-identity-bar');
  if (!host) return () => {};

  const s = stringsFor(lang);
  let showForm = false;
  let message = '';

  const emitSession = (session: UnlockedSession | null): void => {
    window.dispatchEvent(new CustomEvent(SESSION_EVENT, { detail: { session } }));
  };

  const render = (): void => {
    const st = getState();
    const acc = activeAccount();

    if (st.session) {
      loadChain(st.session.pubkeyHex);
      host.innerHTML = `
        <div class="flex flex-wrap items-center gap-3">
          <span class="badge badge-ok">${esc(s.unlocked)}</span>
          <span class="font-bold text-white">${esc(acc?.label ?? 'account')}</span>
          <code class="font-mono text-xs text-cyber-cyan" title="${esc(st.session.pubkeyHex)}">${esc(short(st.session.pubkeyHex))}</code>
          <button data-act="copy" class="rounded-lg border border-white/12 px-2.5 py-1 text-xs font-bold text-white/70 hover:border-cyber-blue/40 hover:text-cyber-blue">${esc(s.copy)}</button>
          <span class="flex-1"></span>
          <button data-act="lock" class="rounded-lg border border-white/12 px-3 py-1.5 text-xs font-bold text-white/70 hover:border-cyber-blue/40 hover:text-cyber-blue">${esc(s.lock)}</button>
        </div>
        ${message ? `<p class="mt-3 text-xs text-cyber-cyan">${esc(message)}</p>` : ''}
      `;
      return;
    }

    const options = st.accounts
      .map(
        (a) =>
          `<option value="${esc(a.id)}"${a.id === st.activeId ? ' selected' : ''}>${esc(a.label)} · ${esc(short(a.id))}</option>`,
      )
      .join('');

    host.innerHTML = `
      <div class="flex flex-wrap items-center gap-3">
        <span class="badge badge-mute">${esc(s.locked)}</span>
        ${
          st.accounts.length > 0
            ? `<label class="text-xs text-white/50">${esc(s.selectAccount)}</label>
               <select data-act="account" class="max-w-[16rem]">${options}</select>`
            : `<span class="text-xs text-white/45">${esc(s.accounts)}: 0</span>`
        }
        <span class="flex-1"></span>
        ${
          showForm
            ? `<button data-act="cancel" class="rounded-lg border border-white/12 px-3 py-1.5 text-xs font-bold text-white/70 hover:text-white">${esc(s.cancel)}</button>`
            : `<button data-act="new" class="rounded-lg border border-cyber-blue/40 bg-cyber-blue/10 px-3 py-1.5 text-xs font-bold text-cyber-blue hover:bg-cyber-blue/20">${esc(s.newIdentity)}</button>`
        }
      </div>

      ${
        showForm
          ? `<div class="mt-4 grid gap-3 border-t border-white/[0.06] pt-4 sm:grid-cols-4">
               <input data-f="label" type="text" placeholder="${esc(s.label)}" />
               <input data-f="pass" type="password" placeholder="${esc(s.passphrase)}" />
               <input data-f="pass2" type="password" placeholder="${esc(s.confirmPassphrase)}" />
               <button data-act="create" class="rounded-lg border border-cyber-blue/50 bg-cyber-blue/15 px-4 py-2 text-sm font-bold text-cyber-blue hover:bg-cyber-blue/25">${esc(s.create)}</button>
             </div>`
          : `<div class="mt-4 flex flex-wrap items-center gap-3 border-t border-white/[0.06] pt-4">
               <input data-f="unlockPass" type="password" class="max-w-[18rem]" placeholder="${esc(s.passphrase)}" />
               <button data-act="unlock" class="rounded-lg border border-cyber-blue/50 bg-cyber-blue/15 px-4 py-2 text-sm font-bold text-cyber-blue hover:bg-cyber-blue/25"${st.accounts.length === 0 ? ' disabled' : ''}>${esc(s.unlock)}</button>
               ${st.accounts.length > 0 ? `<span class="flex-1"></span><button data-act="remove" class="rounded-lg border border-white/10 px-3 py-1.5 text-xs text-white/45 hover:border-cyber-rose/40 hover:text-cyber-red">${esc(s.remove)}</button>` : ''}
             </div>`
      }
      ${message ? `<p class="mt-3 text-xs text-cyber-cyan">${esc(message)}</p>` : ''}
    `;
  };

  const onClick = async (ev: Event): Promise<void> => {
    const el = (ev.target as HTMLElement).closest('[data-act]') as HTMLElement | null;
    if (!el) return;
    const act = el.dataset.act;
    message = '';
    try {
      if (act === 'lock') {
        lock();
        emitSession(null);
      } else if (act === 'new') {
        showForm = true;
      } else if (act === 'cancel') {
        showForm = false;
      } else if (act === 'copy') {
        const pk = getState().session?.pubkeyHex;
        if (pk) {
          await navigator.clipboard.writeText(pk);
          message = s.copied;
        }
      } else if (act === 'create') {
        const label = (host.querySelector('[data-f="label"]') as HTMLInputElement)?.value ?? '';
        const pass = (host.querySelector('[data-f="pass"]') as HTMLInputElement)?.value ?? '';
        const pass2 = (host.querySelector('[data-f="pass2"]') as HTMLInputElement)?.value ?? '';
        if (pass.length < 6) throw new Error(s.passphraseTooShort);
        if (pass !== pass2) throw new Error(s.passphraseMismatch);
        const session = await createIdentity(label, pass);
        showForm = false;
        emitSession(session);
      } else if (act === 'unlock') {
        const pass = (host.querySelector('[data-f="unlockPass"]') as HTMLInputElement)?.value ?? '';
        const id = getState().activeId;
        if (!id) throw new Error(s.unlockFailed);
        const session = await unlock(id, pass);
        emitSession(session);
      } else if (act === 'remove') {
        const id = getState().activeId;
        if (id) removeAccount(id);
      }
    } catch (e) {
      message = e instanceof Error ? e.message : String(e);
    }
    render();
  };

  const onChange = (ev: Event): void => {
    const el = ev.target as HTMLSelectElement;
    if (el.dataset?.act === 'account') {
      selectAccount(el.value);
      emitSession(getState().session);
      render();
    }
  };

  host.addEventListener('click', onClick);
  host.addEventListener('change', onChange);
  const unsub = subscribe(render);
  render();

  // 首次进入若已解锁（同一次客户端会话内换页），广播一次
  emitSession(getState().session);

  return () => {
    host.removeEventListener('click', onClick);
    host.removeEventListener('change', onChange);
    unsub();
  };
}

/**
 * 在控制台页里订阅会话变化；回调立即以当前会话触发一次。
 * 返回清理函数。
 */
export function onSession(fn: (session: UnlockedSession | null) => void): () => void {
  const handler = (ev: Event): void => {
    fn((ev as CustomEvent<{ session: UnlockedSession | null }>).detail.session);
  };
  window.addEventListener(SESSION_EVENT, handler);
  fn(currentSession());
  return () => window.removeEventListener(SESSION_EVENT, handler);
}

/** 渲染一段日志行。 */
export function logLine(hostId: string, text: string, kind: 'info' | 'ok' | 'err' = 'info'): void {
  const host = document.getElementById(hostId);
  if (!host) return;
  const color = kind === 'ok' ? 'text-cyber-blue' : kind === 'err' ? 'text-cyber-red' : 'text-white/60';
  const time = new Date().toLocaleTimeString();
  const row = document.createElement('p');
  row.className = `${color} font-mono text-xs`;
  row.textContent = `[${time}] ${text}`;
  host.prepend(row);
}
