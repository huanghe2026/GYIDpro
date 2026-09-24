#!/usr/bin/env bash
#
# 校验 GeoLife 目录结构并输出统计摘要（标定前自检）。
#
# 期望结构:
#   <root>/<user_id>/Trajectory/*.plt
#
# 只统计文件与目录数量，不读取/打印任何坐标。
#
# 用法:
#   tools/calibration/prepare_dataset.sh --root "tools/calibration/data/GeoLife Trajectories 1.3"

set -euo pipefail

ROOT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --root) ROOT="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "未知参数: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "${ROOT}" ]]; then
  echo "错误：必须指定 --root <数据集根目录>" >&2
  exit 2
fi

if [[ ! -d "${ROOT}" ]]; then
  echo "错误：目录不存在: ${ROOT}" >&2
  exit 1
fi

echo "检查数据集结构: ${ROOT}"

users=0
trajectories=0
users_without_traj=0

# 遍历一级子目录（用户目录）
while IFS= read -r -d '' user_dir; do
  users=$((users + 1))
  traj_dir="${user_dir}/Trajectory"
  if [[ -d "${traj_dir}" ]]; then
    n=$(find "${traj_dir}" -maxdepth 1 -type f -name '*.plt' | wc -l | tr -d ' ')
    trajectories=$((trajectories + n))
  else
    users_without_traj=$((users_without_traj + 1))
  fi
done < <(find "${ROOT}" -mindepth 1 -maxdepth 1 -type d -print0)

echo
echo "  用户目录数:            ${users}"
echo "  PLT 轨迹文件数:        ${trajectories}"
echo "  无 Trajectory 的用户:  ${users_without_traj}"
echo "  磁盘占用:              $(du -sh "${ROOT}" | awk '{print $1}')"

if [[ "${trajectories}" -eq 0 ]]; then
  echo
  echo "错误：未找到任何 .plt 文件，目录结构可能不符（期望 <root>/<user>/Trajectory/*.plt）" >&2
  exit 1
fi

# 标定进入可靠区间需要足够多的轨迹（W7 目标是 200+ 条可用轨迹）
if [[ "${trajectories}" -lt 200 ]]; then
  echo
  echo "警告：轨迹数 < 200，PSD 收敛区间偏弱（草案 §7.4.1 建议 ≥200 条）。" >&2
fi

echo
echo "✓ 结构校验通过，可运行："
echo "  ./target/release/gyid calibrate geolife --data-dir \"${ROOT}\" --control 200 --output /tmp/geolife.json"
