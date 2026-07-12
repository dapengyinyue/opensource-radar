// 与后端 /api/v1 对应的类型。手写，需与 server DTO 保持同步。

export type SourceKind = "github" | "hackernews";

export interface Project {
  id: number;
  dedup_key: string;
  name: string;
  full_name: string | null;
  description: string | null;
  repo_url: string | null;
  homepage_url: string | null;
  language: string | null;
  topics: string[];
  stars: number | null;
  forks: number | null;
  open_issues: number | null;
  hn_points: number | null;
  hn_comment_count: number | null;
  github_created_at: string | null;
  github_updated_at: string | null;
  last_activity_at: string | null;
  source_kinds: string[];
  metadata: Record<string, unknown>;
  first_seen_at: string;
  last_collected_at: string;
  /** stars 增量（仅 rising 榜返回，其他接口不出现） */
  star_delta?: number;
}

export interface ListResponse {
  data: Project[];
  page: number;
  per_page: number;
  total: number;
}

export interface Snapshot {
  id: number;
  project_id: number;
  stars: number | null;
  hn_points: number | null;
  captured_at: string;
}

export interface Facet {
  key: string;
  count: number;
}

export interface SourceStatus {
  source: string;
  last_collected_at: string | null;
  project_count: number;
}

export interface GithubRepoRecord {
  full_name: string;
  node_id: string | null;
}

export interface HnStoryRecord {
  object_id: string;
  hn_url: string;
  linked_url: string | null;
  author: string | null;
  points: number | null;
  comment_count: number | null;
  posted_at: string | null;
}

export interface ProjectSources {
  github: GithubRepoRecord | null;
  hackernews: HnStoryRecord[];
}

export type Sort = "hottest" | "stars" | "recent" | "hn_points" | "rising";
export type Since = "7d" | "30d" | "all";
