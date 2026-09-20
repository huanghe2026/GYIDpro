// Explorer 页占位（Phase 6）
//
// 公开身份 / PoH 浏览器依赖 Verifier 侧 `/v1/explorer` 查询端点，
// 该端点尚未实现（W6 计划标为 future work）。本页仅占位提示。
export default function Explorer() {
  return (
    <div class="bg-white rounded-lg shadow p-8 text-center">
      <h1 class="text-2xl font-bold mb-2">Explorer</h1>
      <p class="text-gray-600 mb-1">
        公开身份 / PoH 浏览器
      </p>
      <p class="text-sm text-gray-400">
        待 Verifier <code class="text-gray-500">/v1/explorer</code> 端点支持后实现（future work）。
      </p>
    </div>
  );
}
