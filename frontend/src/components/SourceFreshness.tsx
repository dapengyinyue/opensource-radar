import { useQuery } from "@tanstack/react-query";
import { getSourcesStatus } from "../api/client";
import { minutesAgo } from "../lib/format";

// 各源期望更新周期（毫秒），超过则标黄提示陈旧。与后端默认调度一致。
const PERIOD_MS: Record<string, number> = {
  github: 60 * 60 * 1000, // 1h
  hackernews: 30 * 60 * 1000, // 30min
};

const LABEL: Record<string, string> = {
  github: "GitHub",
  hackernews: "HackerNews",
};

export default function SourceFreshness() {
  const { data } = useQuery({
    queryKey: ["sourcesStatus"],
    queryFn: getSourcesStatus,
    // 随首页加载，不频繁刷新
    staleTime: 60_000,
  });
  if (!data || data.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border bg-white px-3 py-2 text-xs text-slate-600">
      <span className="text-slate-400">数据时效：</span>
      {data.map((s) => {
        const stale = s.last_collected_at
          ? Date.now() - new Date(s.last_collected_at).getTime() >
            (PERIOD_MS[s.source] ?? Infinity)
          : false;
        return (
          <span key={s.source} className={stale ? "text-amber-600" : ""}>
            {LABEL[s.source] ?? s.source}：
            {s.last_collected_at ? minutesAgo(s.last_collected_at) : "未采集"}
            <span className="text-slate-400">（共 {s.project_count}）</span>
            {stale && <span className="ml-1 font-medium">· 可能延迟</span>}
          </span>
        );
      })}
    </div>
  );
}
