// 在**纯 JS 的内存 EVM**（@ethereumjs/vm）上执行 Solidity 测试。
//
// 为什么这么设计：仓库环境里没有 forge/anvil，但测试逻辑本身是 Solidity。
// 本脚本把 `test/GeoTITRegistry.t.sol` 部署到内存 EVM，逐个调用 `setUp` +
// 每个 `test_*` 函数（内部 `assert` 失败 = revert = 用例失败），做到
// 「一套源码两种跑法」：
//   - forge test                    （Solidity 开发者；测试合约零外部依赖）
//   - node tools/test-inproc.mjs    （无 forge 时的等价验证）
//
// 只用到 EVM 的 runCall：测试内的断言与状态读取全部在 Solidity 里完成，
// JS 侧只负责发交易、判断 revert、统计结果。

import { Common, Hardfork, Mainnet } from "@ethereumjs/common";
import { createAddressFromString, createZeroAddress } from "@ethereumjs/util";
import { createVM } from "@ethereumjs/vm";
import { Interface, getBytes } from "ethers";

import { artifactOf, compileAll } from "./compile.mjs";

const GAS_LIMIT = 10_000_000n;

// 与 foundry.toml 对齐的保守 EVM 版本（合约无 PUSH0/TSTORE 依赖）。
const common = new Common({ chain: Mainnet, hardfork: Hardfork.Paris });
const vm = await createVM({ common });

const ANY_CALLER = createAddressFromString("0x1111111111111111111111111111111111111111");

// @ethereumjs/vm 的 `defaultBlock()` 会把 timestamp 置 0，导致依赖
// `block.timestamp` 的断言失去意义。这里注入一个贴近真实的区块头
// （结构与 defaultBlock() 相同，只是时间/序号/gas 是真实值）。
const TEST_BLOCK = {
  header: {
    number: 1n,
    coinbase: createZeroAddress(),
    timestamp: 1_700_000_000n,
    difficulty: 0n,
    prevRandao: new Uint8Array(32),
    gasLimit: 30_000_000n,
    baseFeePerGas: 0n,
  },
};
const BLOCK_TIMESTAMP = TEST_BLOCK.header.timestamp;

/** 执行一次 EVM 调用；返回 EVMResult。 */
async function evmCall({ caller = ANY_CALLER, to, data, value = 0n }) {
  return vm.evm.runCall({
    caller,
    to,
    data,
    value,
    gasLimit: GAS_LIMIT,
    skipBalance: true,
    block: TEST_BLOCK,
  });
}

/** 部署合约（`to = undefined` + `data = init code`）。 */
async function deploy(bytecodeHex, ctorData = "0x") {
  const data = getBytes(bytecodeHex + ctorData.slice(2));
  const res = await evmCall({ to: undefined, data });
  if (res.execResult.exceptionError !== undefined) {
    throw new Error(`部署失败：${res.execResult.exceptionError.error}`);
  }
  if (!res.createdAddress) throw new Error("部署未返回合约地址");
  return res.createdAddress;
}

const contracts = compileAll();
const testArtifact = artifactOf(
  contracts,
  "test/GeoTITRegistry.t.sol",
  "GeoTITRegistryTest",
);

const iface = new Interface(testArtifact.abi);
const testAddr = await deploy(testArtifact.bytecode);

const fns = testArtifact.abi.filter((e) => e.type === "function");
const tests = fns
  .filter((e) => e.name.startsWith("test_"))
  .map((e) => e.name)
  .sort();
const hasSetUp = fns.some((e) => e.name === "setUp");

if (tests.length === 0) {
  console.error("未找到 test_* 用例");
  process.exit(1);
}

console.log(`GeoTITRegistryTest @ ${testAddr.toString()}`);
console.log(`内存 EVM：@ethereumjs/vm / hardfork=${Hardfork.Paris} / block.timestamp=${BLOCK_TIMESTAMP}`);
console.log(`共 ${tests.length} 个用例（setUp=${hasSetUp ? "每例调用" : "无"}）\n`);

const selector = (name) => getBytes(iface.encodeFunctionData(name));

let passed = 0;
const failures = [];

for (const name of tests) {
  const started = Date.now();
  try {
    if (hasSetUp) {
      const s = await evmCall({ to: testAddr, data: selector("setUp") });
      if (s.execResult.exceptionError !== undefined) {
        throw new Error(`setUp revert: ${s.execResult.exceptionError.error}`);
      }
    }
    const res = await evmCall({ to: testAddr, data: selector(name) });
    const err = res.execResult.exceptionError;
    if (err !== undefined) {
      // 提取 revert 原因（自定义 error 的选择器会出现在 returnValue）
      const ret = res.execResult.returnValue;
      const reason =
        ret && ret.length >= 4 ? `revert data 0x${Buffer.from(ret).toString("hex")}` : err.error;
      throw new Error(reason);
    }
    passed += 1;
    console.log(`  ✓ ${name}  (gas ${res.execResult.executionGasUsed}, ${Date.now() - started}ms)`);
  } catch (e) {
    failures.push(`${name}: ${e.message}`);
    console.log(`  ✗ ${name}  → ${e.message}`);
  }
}

console.log(`\n${passed}/${tests.length} passed`);
if (failures.length > 0) {
  console.error("\n失败用例:");
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log("PASS: GeoTITRegistry 行为测试全部通过（内存 EVM 实跑）");
