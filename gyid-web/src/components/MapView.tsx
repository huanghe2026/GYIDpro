// Leaflet 地图的极简 Solid 封装（命令式 API，避免引入 solid-leaflet 依赖）。
// - 蓝点 + 折线 = 已采集面包屑的原始 GPS 轨迹
// - 红点 = 实时 GPS 定位
// 全部用 circleMarker，绕开 leaflet 默认 marker 图标在 Vite 下的静态资源路径问题。

import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import type { CoordsPoint } from "../stores/chain";

export interface LatLng {
  lat: number;
  lng: number;
  accuracy?: number;
}

export default function MapView(props: {
  coords: CoordsPoint[];
  current: LatLng | null;
}) {
  let container!: HTMLDivElement;
  let map: L.Map | null = null;
  let trackLine: L.Polyline | null = null;
  const trackMarkers: L.CircleMarker[] = [];
  let currentMarker: L.CircleMarker | null = null;
  let accuracyCircle: L.Circle | null = null;
  const [ready, setReady] = createSignal(false);

  onMount(() => {
    map = L.map(container, { zoomControl: true }).setView(
      [39.9042, 116.4074],
      14,
    );
    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution: "© OpenStreetMap contributors",
      maxZoom: 19,
    }).addTo(map);
    setReady(true);
    // 容器尺寸在挂载后确定，通知 leaflet 重新计算
    setTimeout(() => map?.invalidateSize(), 0);
  });

  // 已采集轨迹
  createEffect(() => {
    ready(); // 等地图初始化完成
    const m = map;
    if (!m) return;
    const pts = props.coords.map(
      (c) => [c.lat, c.lng] as [number, number],
    );
    trackLine?.remove();
    trackMarkers.forEach((mk) => mk.remove());
    trackMarkers.length = 0;

    if (pts.length > 0) {
      trackLine = L.polyline(pts, { color: "#2563eb", weight: 3 }).addTo(m);
      pts.forEach((p) => {
        trackMarkers.push(
          L.circleMarker(p, {
            radius: 5,
            color: "#1d4ed8",
            weight: 1.5,
            fillColor: "#3b82f6",
            fillOpacity: 0.75,
          }).addTo(m),
        );
      });
      m.fitBounds(trackLine.getBounds(), {
        padding: [40, 40],
        maxZoom: 16,
      });
    }
  });

  // 当前实时定位
  createEffect(() => {
    ready();
    const m = map;
    if (!m) return;
    const c = props.current;
    currentMarker?.remove();
    accuracyCircle?.remove();
    currentMarker = null;
    accuracyCircle = null;
    if (c) {
      currentMarker = L.circleMarker([c.lat, c.lng], {
        radius: 8,
        color: "#dc2626",
        weight: 2,
        fillColor: "#ef4444",
        fillOpacity: 0.9,
      }).addTo(m);
      if (typeof c.accuracy === "number" && Number.isFinite(c.accuracy)) {
        accuracyCircle = L.circle([c.lat, c.lng], {
          radius: c.accuracy,
          color: "#ef4444",
          weight: 1,
          fillColor: "#ef4444",
          fillOpacity: 0.08,
        }).addTo(m);
      }
      if (props.coords.length === 0) {
        m.setView([c.lat, c.lng], 16);
      }
    }
  });

  onCleanup(() => {
    map?.remove();
    map = null;
  });

  return <div ref={container} class="w-full h-80 rounded-lg overflow-hidden border" />;
}
