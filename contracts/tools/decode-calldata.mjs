// 用编译产物的 ABI 反解一段 calldata（调试用）。
//
//   node tools/decode-calldata.mjs 0x4f65a1cc...
//
// 主要用途：校验 Rust 侧 `trip-core::anchor` 手工编码的 calldata 是否能被
// 权威实现（ethers，按 solc 编译出的 ABI）正确解析成预期参数。

import { Interface } from "ethers";
import { compileAll, artifactOf } from "./compile.mjs";

const hex = process.argv[2];
if (!hex) {
  console.error("用法: node tools/decode-calldata.mjs 0x<calldata>");
  process.exit(1);
}

const contracts = compileAll();
const { abi } = artifactOf(contracts, "src/GeoTITRegistry.sol", "GeoTITRegistry");
const iface = new Interface(abi);

const parsed = iface.parseTransaction({ data: hex });
if (!parsed) {
  console.error("无法解析该 calldata（选择器不在 ABI 中）");
  process.exit(1);
}

console.log(`函数    : ${parsed.signature}`);
console.log(`选择器  : ${parsed.selector}`);
console.log(`参数    :`);
parsed.fragment.inputs.forEach((input, i) => {
  const v = parsed.args[i];
  console.log(`  [${i}] ${input.name} (${input.type}) = ${typeof v === "bigint" ? v.toString() : v}`);
});
