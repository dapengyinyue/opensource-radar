import type { Sort, Since } from "../api/types";

export interface FilterState {
  language: string;
  source: string;
  sort: Sort;
  since: Since;
}

interface Props {
  value: FilterState;
  languages: string[];
  onChange: (next: FilterState) => void;
}

const SORTS: Sort[] = ["hottest", "stars", "recent", "hn_points"];
const SINCES: Since[] = ["7d", "30d", "all"];
const SOURCES = ["all", "github", "hackernews"];

export default function FilterBar({ value, languages, onChange }: Props) {
  const set = (patch: Partial<FilterState>) => onChange({ ...value, ...patch });

  return (
    <div className="flex flex-wrap items-center gap-3 rounded-lg border bg-white p-3">
      <label className="flex items-center gap-1 text-sm">
        语言
        <select
          className="rounded border px-2 py-1"
          value={value.language}
          onChange={(e) => set({ language: e.target.value })}
        >
          <option value="">全部</option>
          {languages.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>
      </label>

      <label className="flex items-center gap-1 text-sm">
        来源
        <select
          className="rounded border px-2 py-1"
          value={value.source}
          onChange={(e) => set({ source: e.target.value })}
        >
          {SOURCES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </label>

      <label className="flex items-center gap-1 text-sm">
        排序
        <select
          className="rounded border px-2 py-1"
          value={value.sort}
          onChange={(e) => set({ sort: e.target.value as Sort })}
        >
          {SORTS.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </label>

      <label className="flex items-center gap-1 text-sm">
        时间
        <select
          className="rounded border px-2 py-1"
          value={value.since}
          onChange={(e) => set({ since: e.target.value as Since })}
        >
          {SINCES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
