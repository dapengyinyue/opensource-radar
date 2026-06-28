interface Props {
  page: number;
  perPage: number;
  total: number;
  onChange: (page: number) => void;
}

/** 简单分页器：上一页 / 当前页 / 总页 / 下一页。 */
export default function Pagination({ page, perPage, total, onChange }: Props) {
  const totalPages = Math.max(1, Math.ceil(total / perPage));
  if (total === 0) return null;
  const prevDisabled = page <= 1;
  const nextDisabled = page >= totalPages;

  return (
    <div className="flex items-center justify-center gap-4 py-2 text-sm">
      <button
        disabled={prevDisabled}
        onClick={() => onChange(page - 1)}
        className="rounded border px-3 py-1 disabled:cursor-not-allowed disabled:text-slate-400 hover:bg-slate-50 disabled:hover:bg-transparent"
      >
        ← 上一页
      </button>
      <span className="tabular-nums text-slate-600">
        第 {page} / {totalPages} 页
      </span>
      <button
        disabled={nextDisabled}
        onClick={() => onChange(page + 1)}
        className="rounded border px-3 py-1 disabled:cursor-not-allowed disabled:text-slate-400 hover:bg-slate-50 disabled:hover:bg-transparent"
      >
        下一页 →
      </button>
    </div>
  );
}
