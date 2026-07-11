// 共享格式化工具。

/** 紧凑数字：1.2k / 3.4M，null → "-" */
export function formatCompact(n: number | null): string {
  if (n === null) return "-";
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

/** 分钟级相对时间："刚刚" / "12 分钟前" / "3 小时前" / "2 天前" ... */
export function minutesAgo(iso: string | null): string {
  if (!iso) return "-";
  const secs = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (secs < 60) return "刚刚";
  if (secs < 3600) return `${Math.floor(secs / 60)} 分钟前`;
  if (secs < 86400) return `${Math.floor(secs / 3600)} 小时前`;
  const days = Math.floor(secs / 86400);
  if (days < 30) return `${days} 天前`;
  if (days < 365) return `${Math.floor(days / 30)} 月前`;
  return `${Math.floor(days / 365)} 年前`;
}

/** 天级相对时间（列表活跃列用，与历史一致）。 */
export function timeAgo(iso: string | null): string {
  if (!iso) return "-";
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86400000);
  if (days <= 0) return "今天";
  if (days < 30) return `${days} 天前`;
  if (days < 365) return `${Math.floor(days / 30)} 月前`;
  return `${Math.floor(days / 365)} 年前`;
}

/** 排序枚举的中文标签。 */
export const SORT_LABELS: Record<string, string> = {
  hottest: "综合热度",
  stars: "Stars 数",
  recent: "最近活跃",
  hn_points: "HN 热度",
  rising: "上升最快",
};

export const SINCE_LABELS: Record<string, string> = {
  "7d": "近 7 天",
  "30d": "近 30 天",
  all: "全部时间",
};
