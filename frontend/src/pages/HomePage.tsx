import { useQuery, keepPreviousData } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { listLanguages, listProjects } from "../api/client";
import type { Sort, Since } from "../api/types";
import FilterBar, { type FilterState } from "../components/FilterBar";
import RankingTable from "../components/RankingTable";
import Pagination from "../components/Pagination";
import SourceFreshness from "../components/SourceFreshness";

const PER_PAGE = 50;

function parseSort(s: string | null): Sort {
  return (["hottest", "stars", "recent", "hn_points"] as const).includes(s as Sort)
    ? (s as Sort)
    : "hottest";
}
function parseSince(s: string | null): Since {
  return (["7d", "30d", "all"] as const).includes(s as Since) ? (s as Since) : "all";
}
function parsePage(s: string | null): number {
  const n = Number(s);
  return Number.isFinite(n) && n >= 1 ? Math.floor(n) : 1;
}

export default function HomePage() {
  const [params, setParams] = useSearchParams();
  const filter: FilterState = {
    language: params.get("language") ?? "",
    source: params.get("source") ?? "all",
    sort: parseSort(params.get("sort")),
    since: parseSince(params.get("since")),
  };
  const q = params.get("q") ?? "";
  const page = parsePage(params.get("page"));

  // 任意筛选改动：重置到第 1 页
  const update = (next: FilterState) => {
    const p = new URLSearchParams();
    if (next.language) p.set("language", next.language);
    if (next.source && next.source !== "all") p.set("source", next.source);
    if (next.sort !== "hottest") p.set("sort", next.sort);
    if (next.since !== "all") p.set("since", next.since);
    setParams(p, { replace: true });
  };

  const setPage = (n: number) => {
    const p = new URLSearchParams(params);
    if (n <= 1) p.delete("page");
    else p.set("page", String(n));
    setParams(p, { replace: true });
  };

  const clearFilters = () => setParams(new URLSearchParams(), { replace: true });

  const hasFilter =
    !!filter.language ||
    (filter.source && filter.source !== "all") ||
    filter.sort !== "hottest" ||
    filter.since !== "all" ||
    !!q;

  const { data: langFacets } = useQuery({
    queryKey: ["languages"],
    queryFn: listLanguages,
  });

  const { data, isPending, isFetching, error } = useQuery({
    queryKey: ["projects", filter, q, page],
    queryFn: () =>
      listProjects({
        page,
        per_page: PER_PAGE,
        q: q || undefined,
        language: filter.language || undefined,
        source: filter.source && filter.source !== "all" ? filter.source : undefined,
        sort: filter.sort,
        since: filter.since,
      }),
    placeholderData: keepPreviousData,
  });

  return (
    <div className="space-y-4">
      <FilterBar value={filter} languages={langFacets ?? []} onChange={update} />
      <SourceFreshness />

      {q && (
        <p className="text-sm text-slate-600">
          搜索 “{q}” ·{" "}
          {data ? `${data.total} 个结果` : "加载中"}
        </p>
      )}

      {isPending && <p className="text-slate-500">加载中…</p>}
      {error && (
        <p className="text-red-600">加载失败：{(error as Error).message}</p>
      )}

      {data && (
        <>
          <div className="flex items-center justify-between text-sm text-slate-500">
            <span>共 {data.total} 个项目</span>
            {isFetching && <span className="text-slate-400">更新中…</span>}
          </div>
          <RankingTable
            projects={data.data}
            returnSearch={params.toString()}
            hasFilter={hasFilter}
            onClearFilters={clearFilters}
          />
          <Pagination
            page={data.page}
            perPage={data.per_page}
            total={data.total}
            onChange={setPage}
          />
        </>
      )}
    </div>
  );
}
