// 部署 GeoTITRegistry 到 Base Sepolia（或任意 EVM）。
//
// 用法：
//   PRIVATE_KEY=0x... VERIFIER=0x... node tools/deploy.mjs
//   RPC_URL=https://sepolia.base.org PRIVATE_KEY=0x... node tools/deploy.mjs
//   node tools/deploy.mjs --dry-run        # 只打印部署参数与 calldata 大小
//
// 说明：
// - `VERIFIER` 是唯一有权限写链的地址（Verifier / 平台中继），
//   缺省用部署者地址，之后可由 owner 用 `setVerifier` 轮换；
// - Base Sepolia chainId = 84532，Base 主网 = 8453（主网部署请显式
//   传 `RPC_URL` 并二次确认）；
// - 部署者需要少量 ETH 付 gas（Sepolia faucet 免费领）。

import { ethers } from "ethers";
import { compileAll, artifactOf } from "./compile.mjs";

const BASE_SEPOLIA_CHAIN_ID = 84532n;

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const rpcArg = args.find((a) => a.startsWith("--rpc="));
const rpc = process.env.RPC_URL ?? (rpcArg ? rpcArg.slice("--rpc=".length) : "https://sepolia.base.org");

const contracts = compileAll();
const { abi, bytecode } = artifactOf(
  contracts,
  "src/GeoTITRegistry.sol",
  "GeoTITRegistry",
);

const pk = process.env.PRIVATE_KEY;
if (!pk) {
  console.error("缺少 PRIVATE_KEY 环境变量（部署者私钥，勿提交到仓库）");
  process.exit(1);
}

let wallet;
try {
  wallet = new ethers.Wallet(pk);
} catch {
  console.error("PRIVATE_KEY 不是合法的 32 字节 hex 私钥");
  process.exit(1);
}

const verifier = process.env.VERIFIER ?? wallet.address;
if (!ethers.isAddress(verifier)) {
  console.error(`VERIFIER 不是合法地址: ${verifier}`);
  process.exit(1);
}

const runtimeBytes = (bytecode.length - 2) / 2;
console.log(`部署者      : ${wallet.address}`);
console.log(`Verifier    : ${verifier}`);
console.log(`RPC         : ${rpc}`);
console.log(`bytecode    : ${runtimeBytes} bytes`);

if (dryRun) {
  console.log("\n--dry-run：未广播。ABI 函数列表：");
  for (const e of abi.filter((x) => x.type === "function")) {
    console.log(`  ${e.name}(${e.inputs.map((i) => i.type).join(",")})`);
  }
  process.exit(0);
}

const provider = new ethers.JsonRpcProvider(rpc);
const network = await provider.getNetwork();
if (network.chainId !== BASE_SEPOLIA_CHAIN_ID) {
  console.warn(
    `⚠ chainId = ${network.chainId}（Base Sepolia 应为 ${BASE_SEPOLIA_CHAIN_ID}），请确认不是误连主网`,
  );
}

const balance = await provider.getBalance(wallet.address);
if (balance === 0n) {
  console.error("部署者余额为 0，无法付 gas（Base Sepolia faucet 可免费领取）");
  process.exit(1);
}

const signer = wallet.connect(provider);
const factory = new ethers.ContractFactory(abi, bytecode, signer);
const contract = await factory.deploy(verifier);
const tx = contract.deploymentTransaction();
console.log(`\n已广播部署交易: ${tx.hash}`);

await contract.waitForDeployment();
const address = await contract.getAddress();
const explorer =
  network.chainId === BASE_SEPOLIA_CHAIN_ID
    ? "https://sepolia.basescan.org"
    : "https://basescan.org";

console.log(`✓ GeoTITRegistry = ${address}`);
console.log(`  chainId : ${network.chainId}`);
console.log(`  verifier: ${verifier}`);
console.log(`  浏览器  : ${explorer}/address/${address}`);
console.log(
  `\n接下来（Verifier 侧）用 Rust CLI 锚定 epoch：\n` +
    `  gyid chain anchor --rpc ${rpc} --registry ${address} --seed <verifier-seed> \\\n` +
    `      --pubkey <hex32> --epoch 0 --merkle <hex32> --unique-cells <n>`,
);
