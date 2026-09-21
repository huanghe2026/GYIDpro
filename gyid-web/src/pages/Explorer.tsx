// Explorer 页：公开身份 / PoH 浏览器（Phase 10 真实现）。
//
// 数据源 GET /v1/explorer（聚合全部 attester 链统计 + PoH 计数）。
// 纯只读浏览页：空态引导、错误提示、手动刷新。

import { createResource, For, Show } from "solid-js";
import { fetchExplorer } from "../lib/verifier";

const short = (h: string) =>
  h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;
const formatTs = (sec: number) => new Date(sec * 1000).toLocaleString();

export default function Explorer() {
  const [info, infoActions] = createResource(fetchExplorer);

  return (
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold">Explorer</h1>
          <p class="text-sm text-gray-500 mt-0.5">
            Verifier 上的公开身份与 PoH 浏览
          </p>
        </div>
        <button
          class="bg-blue-600 text-white text-sm rounded px-3 py-2 hover:bg-blue-700"
          onClick={() => infoActions.refetch()}
        >
          刷新
        </button>
      </div>

      <Show
        when={!info.loading}
        fallback={
          <div class="bg-white rounded-lg shadow p-8 text-center text-gray-500">
            加载中…
          </div>
        }
      >
        <Show when={info.error}>
          <div class="bg-white rounded-lg shadow p-8 text-center">
            <p class="text-red-600 text-sm">
              无法连接 Verifier：{info.error?.message}
            </p>
            <p class="text-xs text-gray-400 mt-2">
              确认 trip-server 已启动（默认 http://localhost:8080）
            </p>
          </div>
        </Show>

        <Show when={!info.error && info()}>
          {(data) => (
            <>
              {/* 全网统计 */}
              <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                <Stat
                  label="公开身份总数"
                  value={String(data().total_identities)}
                />
                <Stat
                  label="面包屑总数"
                  value={String(data().total_breadcrumbs)}
                />
              </div>

              {/* 身份列表 */}
              <Show
                when={data().identities.length > 0}
                fallback={
                  <div class="bg-white rounded-lg shadow p-8 text-center">
                    <p class="text-gray-600 mb-1">Verifier 上还没有公开身份</p>
                    <p class="text-sm text-gray-400">
                      去 Collect 页采集并上传第一组面包屑，或用 CLI / Android 客户端接入。
                    </p>
                  </div>
                }
              >
                <div class="grid gap-3 md:grid-cols-2">
                  <For each={data().identities}>
                    {(id) => (
                      <div class="bg-white rounded-lg shadow p-4 space-y-2">
                        <div class="flex items-center justify-between gap-2">
                          <code
                            class="text-xs bg-gray-100 rounded px-2 py-1 font-mono"
                            title={id.attester}
                          >
                            {short(id.attester)}
                          </code>
                          <Show
                            when={id.poh_count > 0}
                            fallback={
                              <span class="text-xs text-gray-400">
                                暂无 PoH
                              </span>
                            }
                          >
                            <span class="text-xs bg-green-50 text-green-700 rounded px-2 py-0.5">
                              PoH × {id.poh_count}
                            </span>
                          </Show>
                        </div>
                        <div class="grid grid-cols-3 gap-2 text-xs">
                          <div>
                            <div class="text-gray-500">面包屑</div>
                            <div class="font-medium">{id.breadcrumb_count}</div>
                          </div>
                          <div>
                            <div class="text-gray-500">唯一 H3 cell</div>
                            <div class="font-medium">{id.unique_cells}</div>
                          </div>
                          <div>
                            <div class="text-gray-500">链头</div>
                            <div
                              class="font-medium font-mono"
                              title={id.chain_head}
                            >
                              {short(id.chain_head)}
                            </div>
                          </div>
                        </div>
                        <div class="text-xs text-gray-400">
                          最近活跃：{formatTs(id.last_ts)}
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </>
          )}
        </Show>
      </Show>
    </div>
  );
}

function Stat(props: { label: string; value: string }) {
  return (
    <div class="bg-white rounded-lg shadow px-4 py-3">
      <div class="text-xs text-gray-500">{props.label}</div>
      <div class="text-xl font-semibold mt-0.5">{props.value}</div>
    </div>
  );
}
