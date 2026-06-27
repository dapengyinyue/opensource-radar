import type { Snapshot } from "../api/types";

interface Props {
  snapshots: Snapshot[];
  width?: number;
  height?: number;
}

// 取 stars（无则 hn_points）画一条简易折线 sparkline。
export default function MetricSparkline({ snapshots, width = 240, height = 48 }: Props) {
  const pts = [...snapshots].reverse(); // 按时间升序
  const values = pts.map((s) => s.stars ?? s.hn_points ?? 0);
  if (values.length < 2) {
    return <p className="text-sm text-slate-500">快照不足，暂无趋势。</p>;
  }
  const max = Math.max(...values);
  const min = Math.min(...values);
  const range = max - min || 1;
  const stepX = width / (values.length - 1);
  const coords = values.map((v, i) => {
    const x = i * stepX;
    const y = height - ((v - min) / range) * height;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  return (
    <svg width={width} height={height} className="block">
      <polyline
        fill="none"
        stroke="#2563eb"
        strokeWidth={2}
        points={coords.join(" ")}
      />
    </svg>
  );
}
