// 路由：公开 TRIP 门户（/、/protocol、/architecture）+ 功能控制台（/console/*）。
import { Router, Route, Navigate } from "@solidjs/router";
import PortalLayout from "./components/PortalLayout";
import ConsoleLayout from "./components/ConsoleLayout";
import Home from "./pages/site/Home";
import Protocol from "./pages/site/Protocol";
import Architecture from "./pages/site/Architecture";
import Identity from "./pages/Identity";
import Collect from "./pages/Collect";
import Import from "./pages/Import";
import Verify from "./pages/Verify";
import Certificates from "./pages/Certificates";
import Explorer from "./pages/Explorer";

/**
 * 客户端路由 base：与 Vite 的 `base` 保持一致。
 *
 * - 默认构建 `base: "./"`（相对）→ 路由 base 用 `/`（现状不变）；
 * - 子路径部署 `VITE_BASE_PATH=/app/` → 路由 base 用 `/app`（去掉尾斜杠）。
 *
 * 这样同一份代码既能嵌入任意静态目录，也能挂在 `https://host/app/` 下。
 */
const routerBase = (() => {
  const raw = import.meta.env.BASE_URL;
  if (!raw.startsWith("/")) return "/";
  return raw.replace(/\/+$/, "") || "/";
})();

export default function App() {
  return (
    <Router base={routerBase}>
      {/* 公开门户 */}
      <Route path="/" component={PortalLayout}>
        <Route path="/" component={Home} />
        <Route path="/protocol" component={Protocol} />
        <Route path="/architecture" component={Architecture} />
      </Route>

      {/* 功能控制台 */}
      <Route path="/console" component={ConsoleLayout}>
        <Route path="/" component={Identity} />
        <Route path="/collect" component={Collect} />
        <Route path="/import" component={Import} />
        <Route path="/verify" component={Verify} />
        <Route path="/certificates" component={Certificates} />
        <Route path="/explorer" component={Explorer} />
      </Route>

      {/* 旧路由兼容：Phase 3-10 的 /collect 等书签跳到控制台 */}
      <Route path="/collect" component={() => <Navigate href="/console/collect" />} />
      <Route path="/import" component={() => <Navigate href="/console/import" />} />
      <Route path="/verify" component={() => <Navigate href="/console/verify" />} />
      <Route path="/certificates" component={() => <Navigate href="/console/certificates" />} />
      <Route path="/explorer" component={() => <Navigate href="/console/explorer" />} />
      <Route path="*" component={() => <Navigate href="/" />} />
    </Router>
  );
}
