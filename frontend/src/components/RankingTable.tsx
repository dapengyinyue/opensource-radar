import { Link } from "react-router-dom";
import type { Project } from "../api/types";
import SourceBadge from "./SourceBadge";

function fmt(n: number | null): string {
  if (n === null) return "-";
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

function timeAgo(iso: string | null): string {
  if (!iso) return "-";
  const d = new Date(iso);
  const days = Math.floor((Date.now() - d.getTime()) / 86400000);
  if (days <= 0) return "今天";
  if (days < 30) return `${days} 天前`;
  if (days < 365) return `${Math.floor(days / 30)} 月前`;
  return `${Math.floor(days / 365)} 年前`;
}

export default function RankingTable({ projects }: { projects: Project[] }) {
  if (projects.length === 0) {
    return <p className="py-8 text-center text-slate-500">暂无数据，先触发一次采集。</p>;
  }
  return (
    <div className="overflow-x-auto rounded-lg border bg-white">
      <table className="w-full text-sm">
        <thead className="bg-slate-100 text-left text-slate-600">
          <tr>
            <th className="px-3 py-2">项目</th>
            <th className="px-3 py-2">来源</th>
            <th className="px-3 py-2 text-right">Stars</th>
            <th className="px-3 py-2 text-right">HN</th>
            <th className="px-3 py-2">语言</th>
            <th className="px-3 py-2">活跃</th>
          </tr>
        </thead>
        <tbody>
          {projects.map((p) => (
            <tr key={p.id} className="border-t hover:bg-slate-50">
              <td className="px-3 py-2">
                <Link to={`/projects/${p.id}`} className="font-medium text-blue-600 hover:underline">
                  {p.full_name ?? p.name}
                </Link>
                {p.description && (
                  <div className="max-w-md truncate text-slate-500">{p.description}</div>
                )}
              </td>
              <td className="px-3 py-2">
                <div className="flex gap-1">
                  {p.source_kinds.map((s) => (
                    <SourceBadge key={s} source={s} />
                  ))}
                </div>
              </td>
              <td className="px-3 py-2 text-right tabular-nums">{fmt(p.stars)}</td>
              <td className="px-3 py-2 text-right tabular-nums">{fmt(p.hn_points)}</td>
              <td className="px-3 py-2">{p.language ?? "-"}</td>
              <td className="px-3 py-2 text-slate-500">{timeAgo(p.last_activity_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
