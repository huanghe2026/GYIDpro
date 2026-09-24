#!/usr/bin/env bash
#
# 下载并解压 Microsoft Research GeoLife 轨迹数据集（W7 标定用）。
#
# 数据集体积约 1.9 GB（解压后更大），**不会**被提交进版本库
# （见同目录 .gitignore）。原始 GPS 只在本机参与量化，脚本不打印任何坐标。
#
# 用法:
#   tools/calibration/download_geolife.sh                 # 默认下载到 tools/calibration/data
#   tools/calibration/download_geolife.sh --dest /data/geolife
#   GEOLIFE_URL=<镜像地址> tools/calibration/download_geolife.sh
#
# 官方下载页（若默认链接失效请从此处获取最新直链）:
#   https://www.microsoft.com/en-us/download/details.aspx?id=52367
#
# 完整性校验:
#   脚本用 `unzip -t` 做结构完整性测试。若你有官方发布的 SHA-256，
#   可通过 `GEOLIFE_SHA256=<hex>` 传入，脚本会逐字节核对。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_DEST="${SCRIPT_DIR}/data"

# GeoLife 1.3 的公开直链（微软 CDN）。若失效，用 GEOLIFE_URL 覆盖。
DEFAULT_URL="https://download.microsoft.com/download/9/6/B/96BB11D1-FE82-44DE-A139-6A100966CE30/Geolife%20Trajectories%201.3.zip"

DEST="${DEFAULT_DEST}"
URL="${GEOLIFE_URL:-${DEFAULT_URL}}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dest) DEST="$2"; shift 2 ;;
    --url)  URL="$2";  shift 2 ;;
    -h|--help)
      sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "未知参数: $1" >&2; exit 2 ;;
  esac
done

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "错误：缺少依赖 '$1'" >&2; exit 1; }
}
need curl
need unzip

mkdir -p "${DEST}"
ZIP="${DEST}/geolife.zip"

if [[ -f "${ZIP}" ]]; then
  echo "已存在压缩包，跳过下载: ${ZIP}"
else
  echo "下载 GeoLife（约 1.9 GB）到 ${ZIP} …"
  echo "  来源: ${URL}"
  if ! curl -fL --retry 3 --retry-delay 5 -o "${ZIP}" "${URL}"; then
    echo "错误：下载失败。可能是链接失效或网络受限。" >&2
    echo "  请到 https://www.microsoft.com/en-us/download/details.aspx?id=52367" >&2
    echo "  手动下载后放到 ${ZIP}，或用 --url / GEOLIFE_URL 指定镜像。" >&2
    rm -f "${ZIP}"
    exit 1
  fi
fi

if [[ -n "${GEOLIFE_SHA256:-}" ]]; then
  echo "校验 SHA-256 …"
  actual="$(sha256sum "${ZIP}" | awk '{print $1}')"
  if [[ "${actual}" != "${GEOLIFE_SHA256}" ]]; then
    echo "错误：SHA-256 不匹配" >&2
    echo "  期望: ${GEOLIFE_SHA256}" >&2
    echo "  实际: ${actual}" >&2
    exit 1
  fi
  echo "  ✓ 校验通过"
fi

echo "测试压缩包完整性（unzip -t）…"
if ! unzip -tq "${ZIP}" >/dev/null; then
  echo "错误：压缩包损坏（unzip -t 失败）" >&2
  exit 1
fi

echo "解压到 ${DEST} …"
unzip -q -o "${ZIP}" -d "${DEST}"

# 定位解压后的根目录（通常名为 GeoLife Trajectories 1.3/）
ROOT="$(find "${DEST}" -maxdepth 2 -type d -name 'Geolife*' -o -maxdepth 2 -type d -name 'GeoLife*' 2>/dev/null | head -n1 || true)"
if [[ -z "${ROOT}" ]]; then
  # 回退：找含数字命名用户子目录的目录
  ROOT="$(find "${DEST}" -maxdepth 2 -type d -regex '.*/[0-9]+$' 2>/dev/null | head -n1 | xargs -r dirname || true)"
fi

if [[ -z "${ROOT}" ]]; then
  echo "警告：未能自动识别数据集根目录，请手动检查 ${DEST}" >&2
  exit 1
fi

echo
echo "✓ 完成"
echo "  数据集根目录: ${ROOT}"
echo "  磁盘占用:     $(du -sh "${DEST}" | awk '{print $1}')"
echo
echo "下一步："
echo "  tools/calibration/prepare_dataset.sh --root \"${ROOT}\""
echo "  ./target/release/gyid calibrate geolife --data-dir \"${ROOT}\" --control 200 --output /tmp/geolife.json"
