import { Link } from "react-router-dom";
import type { Project } from "../api/types";
import SourceBadge from "./SourceBadge";
import { formatCompact, timeAgo } from "../lib/format";

interface Props {
  projects: Project[];
  /** 当前首页筛选的 URL search 串，进入详情时携带，返回时还原（P0-2） */
  returnSearch: string;
  /** 是否处于筛选状态（用于区分空态文案，P0-3） */
  hasFilter: boolean;
  /** 清空筛选回调（空态用，P0-3） */
  onClearFilters?: () => void;
}

export default function RankingTable({ projects, returnSearch, hasFilter, onClearFilters }: Props) {
  if (projects.length === 0) {
    return (
      <div className="rounded-lg border bg-white py-10 text-center text-slate-500">
        {hasFilter ? (
          <div className="space-y-2">
            <p>当前筛选无匹配项目。</p>
            <p className="text-sm">试试清空筛选或放宽时间范围。</p>
            {onClearFilters && (
              <button
                onClick={onClearFilters}
                className="rounded border border-blue-600 px-3 py-1 text-sm text-blue-600 hover:bg-blue-50"
              >
                清空筛选
              </button>
            )}
          </div>
        ) : (
          <p>暂无数据，数据将在下次定时采集后出现。</p>
        )}
      </div>
    );
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
                <Link
                  to={{ pathname: `/projects/${p.id}`, search: returnSearch }}
                  className="font-medium text-blue-600 hover:underline"
                >
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
              <td className="px-3 py-2 text-right tabular-nums">
                {formatCompact(p.stars)}
                {p.star_delta != null && p.star_delta > 0 && (
                  <span className="ml-1 text-xs font-medium text-emerald-600">
                    +{formatCompact(p.star_delta)}
                  </span>
                )}
              </td>
              <td className="px-3 py-2 text-right tabular-nums">{formatCompact(p.hn_points)}</td>
              <td className="px-3 py-2">{p.language ?? "-"}</td>
              <td className="px-3 py-2 text-slate-500">{timeAgo(p.last_activity_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
