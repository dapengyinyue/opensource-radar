import { useQuery } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { listLanguages, listProjects } from "../api/client";
import type { Sort, Since } from "../api/types";
import FilterBar, { type FilterState } from "../components/FilterBar";
import RankingTable from "../components/RankingTable";

function parseSort(s: string | null): Sort {
  return (["hottest", "stars", "recent", "hn_points"] as const).includes(s as Sort)
    ? (s as Sort)
    : "hottest";
}
function parseSince(s: string | null): Since {
  return (["7d", "30d", "all"] as const).includes(s as Since) ? (s as Since) : "all";
}

export default function HomePage() {
  const [params, setParams] = useSearchParams();
  const filter: FilterState = {
    language: params.get("language") ?? "",
    source: params.get("source") ?? "all",
    sort: parseSort(params.get("sort")),
    since: parseSince(params.get("since")),
  };

  const update = (next: FilterState) => {
    const p = new URLSearchParams();
    if (next.language) p.set("language", next.language);
    if (next.source && next.source !== "all") p.set("source", next.source);
    if (next.sort !== "hottest") p.set("sort", next.sort);
    if (next.since !== "all") p.set("since", next.since);
    setParams(p, { replace: true });
  };

  const { data: langFacets } = useQuery({
    queryKey: ["languages"],
    queryFn: listLanguages,
  });
  const languages = (langFacets ?? []).map((f) => f.key);

  const { data, isLoading, error } = useQuery({
    queryKey: ["projects", filter],
    queryFn: () =>
      listProjects({
        page: 1,
        per_page: 50,
        language: filter.language || undefined,
        source: filter.source && filter.source !== "all" ? filter.source : undefined,
        sort: filter.sort,
        since: filter.since,
      }),
  });

  return (
    <div className="space-y-4">
      <FilterBar value={filter} languages={languages} onChange={update} />
      {isLoading && <p className="text-slate-500">加载中…</p>}
      {error && <p className="text-red-600">加载失败：{(error as Error).message}</p>}
      {data && (
        <>
          <p className="text-sm text-slate-500">共 {data.total} 个项目</p>
          <RankingTable projects={data.data} />
        </>
      )}
    </div>
  );
}
