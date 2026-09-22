// GeoTITRegistry 只读客户端：直接 JSON-RPC eth_call，零第三方依赖。
//
// 合约：contracts/src/GeoTITRegistry.sol（MIT），当前目标网络 Base Sepolia。
// 选择器由 contracts 侧 ethers（keccak256 前 4 字节）计算，写函数选择器已与
// contracts/README.md 记录交叉核对一致。
//
// 配置（构建期注入，见 vite.config.ts）：
//   VITE_RPC_URL           JSON-RPC 端点（缺省 Base Sepolia 公共 RPC）
//   VITE_REGISTRY_ADDRESS  合约地址；未配置时 registryConfigured() = false，
//                          控制台不渲染链上卡片（功能整体静默关闭）。

const RPC_URL: string =
  import.meta.env.VITE_RPC_URL ?? "https://sepolia.base.org";
const REGISTRY: string = (
  import.meta.env.VITE_REGISTRY_ADDRESS ?? ""
).trim();

/** Base Sepolia 区块浏览器（与合约 README 的查证链接一致）。 */
const EXPLORER = "https://sepolia.basescan.org";

// eth_call 选择器（keccak256(signature) 前 4 字节）
const SEL_IDENTITY_OF = "0x4fc9c91a"; // identityOf(bytes32)
const SEL_HANDLE_OF = "0x0c1a880a"; // handleOf(bytes32)

/** 是否已配置链上登记簿地址（未配置则整个功能关闭）。 */
export function registryConfigured(): boolean {
  return /^0x[a-fA-F0-9]{40}$/.test(REGISTRY);
}

export function registryAddress(): string {
  return REGISTRY;
}

export function explorerAddressUrl(addr: string): string {
  return `${EXPLORER}/address/${addr}`;
}

async function ethCall(data: string): Promise<string> {
  const res = await fetch(RPC_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "eth_call",
      params: [{ to: REGISTRY, data }, "latest"],
    }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const json = (await res.json()) as {
    result?: string;
    error?: { message?: string };
  };
  if (json.error) throw new Error(json.error.message ?? "RPC error");
  return json.result ?? "0x";
}

/** 32 字节 hex 左填充为一个 ABI word（Ed25519 公钥即 bytes32，无需转换）。 */
function padWord(hex: string): string {
  const h = hex.replace(/^0x/, "").toLowerCase();
  if (h.length !== 64) throw new Error("expected 32-byte hex");
  return h;
}

/** GeoTITRegistry.identityOf + handleOf 的合并视图。 */
export interface ChainIdentity {
  registered: boolean;
  registeredAt: number; // unix 秒
  epochCount: number; // 已锚定 epoch 数
  lastEpoch: number; // 最近 epoch 序号
  lastUniqueCells: number; // 最近 epoch 的唯一 cell 数
  handleClaimedAt: number; // unix 秒（0 = 未声明）
  handle: string; // 空串 = 未声明
}

function wordUint(hex: string): number {
  return Number(BigInt("0x" + hex));
}

/** 查询某身份公钥在 GeoTITRegistry 上的登记状态（未登记也正常返回，不抛错）。 */
export async function fetchChainIdentity(
  pubkeyHex: string,
): Promise<ChainIdentity> {
  const key = padWord(pubkeyHex);
  const [rawTuple, rawHandle] = await Promise.all([
    ethCall(SEL_IDENTITY_OF + key),
    ethCall(SEL_HANDLE_OF + key),
  ]);

  // identityOf 返回 6 个 ABI word：
  // (bool registered, uint64 registeredAt, uint32 epochCount,
  //  uint64 lastEpoch, uint32 lastUniqueCells, uint64 handleClaimedAt)
  // 对非合约地址的 eth_call 会返回空 0x，按全零（未登记）处理。
  const hex = rawTuple.slice(2).padEnd(6 * 64, "0");
  const words = Array.from({ length: 6 }, (_, i) =>
    hex.slice(i * 64, (i + 1) * 64),
  );
  const info: ChainIdentity = {
    registered: words[0] !== "0".repeat(64),
    registeredAt: wordUint(words[1]),
    epochCount: wordUint(words[2]),
    lastEpoch: wordUint(words[3]),
    lastUniqueCells: wordUint(words[4]),
    handleClaimedAt: wordUint(words[5]),
    handle: "",
  };

  // handleOf 返回动态 string：word0=offset(0x20) word1=length，其后是 UTF-8 字节
  const hh = rawHandle.slice(2);
  if (hh.length >= 128) {
    const len = parseInt(hh.slice(64, 128), 16);
    if (Number.isFinite(len) && len > 0) {
      const dataHex = hh.slice(128, 128 + len * 2);
      const bytes = new Uint8Array(dataHex.length / 2);
      for (let i = 0; i < bytes.length; i++) {
        bytes[i] = parseInt(dataHex.slice(i * 2, i * 2 + 2), 16);
      }
      info.handle = new TextDecoder().decode(bytes);
    }
  }

  return info;
}
