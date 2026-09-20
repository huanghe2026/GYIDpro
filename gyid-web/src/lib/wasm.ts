// WASM 动态加载器：先 await loadWasm() 再调用任何导出函数
import initWasm, * as wasm from "../../wasm/gyid_wasm.js";

let loaded = false;
let initPromise: Promise<typeof wasm> | null = null;

/**
 * 加载并初始化 gyid-wasm 模块（幂等，多次调用安全）。
 * 返回 wasm 模块的所有导出函数。
 */
export async function loadWasm() {
  if (loaded) return wasm;
  if (!initPromise) {
    initPromise = initWasm().then(() => {
      loaded = true;
      return wasm;
    });
  }
  return initPromise;
}

export type {
  KeypairJs,
  BreadcrumbJs,
  LivenessChallengeJs,
  LivenessResponseJs,
  PohInfoJs,
} from "../../wasm/gyid_wasm.d.ts";
