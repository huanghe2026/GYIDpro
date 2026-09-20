// solc-js 编译 GeoTITRegistry（含测试合约），产出 artifacts/*.json。
//
// 用途：环境里没有 forge 时也能编译 + 拿到 ABI/bytecode 部署；
// 同时打印 4 字节选择器，供 Rust 侧 `trip-core::anchor` 交叉校验。
//
// 设置与 foundry.toml 保持一致（solc 0.8.37 / evm paris / optimizer 200），
// 保证两条路径产出同样的字节码。

import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import solc from "solc";
import { id } from "ethers";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

const SOURCES = {
  "src/GeoTITRegistry.sol": join(root, "src/GeoTITRegistry.sol"),
  "test/GeoTITRegistry.t.sol": join(root, "test/GeoTITRegistry.t.sol"),
};

const EVM_VERSION = "paris";
const OPTIMIZER_RUNS = 200;

function readSources() {
  const out = {};
  for (const [unit, path] of Object.entries(SOURCES)) {
    out[unit] = { content: readFileSync(path, "utf8") };
  }
  return out;
}

export function compileAll() {
  const compile = typeof solc.compile === "function" ? solc.compile : solc.default.compile;
  const input = {
    language: "Solidity",
    sources: readSources(),
    settings: {
      evmVersion: EVM_VERSION,
      optimizer: { enabled: true, runs: OPTIMIZER_RUNS },
      outputSelection: {
        "*": { "*": ["abi", "evm.bytecode.object", "evm.deployedBytecode.object"] },
      },
    },
  };

  const raw = JSON.parse(compile(JSON.stringify(input)));

  const errors = (raw.errors ?? []).filter((e) => e.severity === "error");
  if (errors.length > 0) {
    for (const e of errors) console.error(e.formattedMessage);
    throw new Error(`solc 编译失败（${errors.length} 个错误）`);
  }
  const warnings = (raw.errors ?? []).filter((e) => e.severity === "warning");
  for (const w of warnings) console.warn(w.formattedMessage.trim());

  return raw.contracts;
}

/** 取某个合约的 {abi, bytecode, deployedBytecode}。 */
export function artifactOf(contracts, unit, name) {
  const c = contracts[unit]?.[name];
  if (!c) throw new Error(`artifact not found: ${unit}:${name}`);
  return {
    abi: c.abi,
    bytecode: `0x${c.evm.bytecode.object}`,
    deployedBytecode: `0x${c.evm.deployedBytecode.object}`,
  };
}

// 打印 4 字节选择器（与 trip-core::anchor 的常量对照）
const SELECTORS = [
  "register(bytes32,string)",
  "anchorEpoch(bytes32,uint64,bytes32,uint32)",
  "claimHandle(bytes32,string,uint64,uint64)",
  "epochKey(bytes32,uint64)",
  "epochRoot(bytes32,uint64)",
];

if (import.meta.url === `file://${process.argv[1]}`) {
  const contracts = compileAll();
  const main = artifactOf(contracts, "src/GeoTITRegistry.sol", "GeoTITRegistry");

  const outDir = join(root, "artifacts");
  mkdirSync(outDir, { recursive: true });
  const payload = {
    contractName: "GeoTITRegistry",
    compiler: { version: "0.8.37", evmVersion: EVM_VERSION, optimizerRuns: OPTIMIZER_RUNS },
    ...main,
  };
  writeFileSync(join(outDir, "GeoTITRegistry.json"), `${JSON.stringify(payload, null, 2)}\n`);
  writeFileSync(
    join(outDir, "GeoTITRegistryTest.json"),
    `${JSON.stringify(
      {
        contractName: "GeoTITRegistryTest",
        ...artifactOf(contracts, "test/GeoTITRegistry.t.sol", "GeoTITRegistryTest"),
      },
      null,
      2,
    )}\n`,
  );

  const runtimeBytes = (main.deployedBytecode.length - 2) / 2;
  console.log(`✓ 编译通过  GeoTITRegistry runtime = ${runtimeBytes} bytes`);
  console.log(`✓ artifacts/GeoTITRegistry.json 已写出`);
  console.log("选择器（keccak256 前 4 字节）:");
  for (const sig of SELECTORS) {
    console.log(`  ${sig}  ${selectorOf(sig)}`);
  }
}

/** 计算函数选择器（keccak256 前 4 字节）。 */
export function selectorOf(signature) {
  // ethers 的 id() = keccak256(utf8)；取前 4 字节
  return id(signature).slice(0, 10);
}
