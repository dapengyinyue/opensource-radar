import type { Snapshot } from "../api/types";

interface Props {
  snapshots: Snapshot[];
  width?: number;
  height?: number;
}

interface Series {
  label: string;
  color: string;
  points: { x: number; y: number }[];
  first: number;
  last: number;
}

/// 按指标分线：stars 与 hn_points 各自只取该指标非 null 的快照，
/// 避免 stars(万级) 与 hn_points(百级) 混在一条线上产生无意义锯齿。
/// 每条线内部按时间归一化到 [0, height]。
function buildSeries(
  snapshots: Snapshot[],
  key: "stars" | "hn_points",
  label: string,
  color: string,
  width: number,
  height: number,
): Series | null {
  const pts = [...snapshots]
    .filter((s) => s[key] !== null && s[key] !== undefined)
    .reverse(); // 时间升序
  if (pts.length < 2) return null;
  const values = pts.map((s) => s[key] as number);
  const max = Math.max(...values);
  const min = Math.min(...values);
  const range = max - min || 1;
  const stepX = width / (values.length - 1);
  const points = values.map((v, i) => ({
    x: i * stepX,
    y: height - ((v - min) / range) * height,
  }));
  return { label, color, points, first: values[0], last: values[values.length - 1] };
}

export default function MetricSparkline({ snapshots, width = 280, height = 56 }: Props) {
  if (snapshots.length < 2) {
    return <p className="text-sm text-slate-500">快照不足，暂无趋势。</p>;
  }

  const series = [
    buildSeries(snapshots, "stars", "Stars", "#2563eb", width, height),
    buildSeries(snapshots, "hn_points", "HN", "#ea580c", width, height),
  ].filter((s): s is Series => s !== null);

  if (series.length === 0) {
    return <p className="text-sm text-slate-500">快照无指标数据。</p>;
  }

  return (
    <div className="space-y-2">
      <svg width={width} height={height} className="block" role="img" aria-label="指标趋势">
        {series.map((s) => (
          <polyline
            key={s.label}
            fill="none"
            stroke={s.color}
            strokeWidth={2}
            points={s.points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ")}
          />
        ))}
      </svg>
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-slate-600">
        {series.map((s) => (
          <span key={s.label}>
            <span
              className="mr-1 inline-block h-2 w-2 align-middle rounded-full"
              style={{ backgroundColor: s.color }}
            />
            {s.label}：{s.first} → {s.last}
          </span>
        ))}
      </div>
    </div>
  );
}
