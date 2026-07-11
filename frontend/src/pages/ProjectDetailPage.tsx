import { useQuery } from "@tanstack/react-query";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { getProject, getProjectSources, listProjects, listSnapshots } from "../api/client";
import SourceBadge from "../components/SourceBadge";
import MetricSparkline from "../components/MetricSparkline";
import { formatCompact, minutesAgo, timeAgo } from "../lib/format";
import type { HnStoryRecord } from "../api/types";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex gap-2 py-1">
      <span className="w-28 shrink-0 text-slate-500">{label}</span>
      <span className="flex-1">{children}</span>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-lg border bg-white p-4">
      <h2 className="mb-2 font-medium">{title}</h2>
      {children}
    </div>
  );
}

function HnStoryRow({ s }: { s: HnStoryRecord }) {
  return (
    <div className="border-t py-2 first:border-t-0">
      <div className="flex items-center gap-2 text-sm">
        <a
          className="font-medium text-orange-700 hover:underline"
          href={s.hn_url}
          target="_blank"
          rel="noreferrer"
        >
          查看 HN 讨论 ↗
        </a>
        <span className="text-slate-400">·</span>
        <span className="text-slate-600">{formatCompact(s.points)} 分</span>
        <span className="text-slate-400">·</span>
        <span className="text-slate-600">{s.comment_count ?? "-"} 评论</span>
      </div>
      <div className="text-xs text-slate-500">
        {s.author ? `by ${s.author} · ` : ""}
        {s.posted_at ? timeAgo(s.posted_at) : ""}
        {s.linked_url && (
          <>
            {" · "}
            <a
              className="text-blue-600 hover:underline"
              href={s.linked_url}
              target="_blank"
              rel="noreferrer"
            >
              外链
            </a>
          </>
        )}
      </div>
    </div>
  );
}

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const numId = Number(id);
  const [searchParams] = useSearchParams();
  const returnSearch = searchParams.toString();

  const { data: project, isLoading, error } = useQuery({
    queryKey: ["project", numId],
    queryFn: () => getProject(numId),
    enabled: Number.isFinite(numId),
  });
  const { data: snapshots } = useQuery({
    queryKey: ["snapshots", numId],
    queryFn: () => listSnapshots(numId),
    enabled: Number.isFinite(numId),
  });
  const { data: sources } = useQuery({
    queryKey: ["projectSources", numId],
    queryFn: () => getProjectSources(numId),
    enabled: Number.isFinite(numId),
  });

  // 相关项目：取第一个 topic 查同话题 hottest TOP5
  const relatedTopic = project?.topics[0];
  const { data: related } = useQuery({
    queryKey: ["relatedProjects", relatedTopic, numId],
    queryFn: () => listProjects({ topic: relatedTopic, sort: "hottest", per_page: 6 }),
    enabled: !!relatedTopic,
  });
  const relatedProjects = (related?.data ?? []).filter((p) => p.id !== numId).slice(0, 5);

  if (isLoading) return <p className="text-slate-500">加载中…</p>;
  if (error) return <p className="text-red-600">加载失败：{(error as Error).message}</p>;
  if (!project) return <p className="text-slate-500">未找到项目。</p>;

  const hasGithub = project.source_kinds.includes("github");
  const hasHn = project.source_kinds.includes("hackernews");
  const isCrossSource = hasGithub && hasHn;

  return (
    <div className="space-y-6">
      <div>
        <Link
          to={{ pathname: "/", search: returnSearch }}
          className="text-sm text-blue-600 hover:underline"
        >
          ← 返回榜单
        </Link>
      </div>

      <div className="rounded-lg border bg-white p-4">
        <div className="flex items-center gap-2">
          <h1 className="text-xl font-semibold">{project.full_name ?? project.name}</h1>
          {project.source_kinds.map((s) => (
            <SourceBadge key={s} source={s} />
          ))}
        </div>
        {project.description && <p className="mt-1 text-slate-600">{project.description}</p>}
        {isCrossSource && (
          <p className="mt-2 text-xs text-slate-500">
            ⓘ 同时被 GitHub 与 HackerNews 收录
          </p>
        )}

        <div className="mt-4 grid grid-cols-1 gap-x-8 md:grid-cols-2">
          <Field label="语言">
            {project.language ? (
              <Link
                to={`/?language=${encodeURIComponent(project.language)}`}
                className="text-blue-600 hover:underline"
              >
                {project.language}
              </Link>
            ) : (
              "-"
            )}
          </Field>
          <Field label="Open Issues">{project.open_issues ?? "-"}</Field>
          <Field label="Topics">
            <div className="flex flex-wrap gap-1">
              {project.topics.map((t) => (
                <Link
                  key={t}
                  to={`/?topic=${encodeURIComponent(t)}`}
                  className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-blue-600 hover:bg-blue-50 hover:underline"
                >
                  {t}
                </Link>
              ))}
            </div>
          </Field>
          <Field label="活跃时间">
            {project.last_activity_at ? new Date(project.last_activity_at).toLocaleString() : "-"}
          </Field>
          <Field label="数据采集">{minutesAgo(project.last_collected_at)}</Field>
          <Field label="仓库">
            {project.repo_url ? (
              <a
                className="text-blue-600 hover:underline"
                href={project.repo_url}
                target="_blank"
                rel="noreferrer"
              >
                {project.repo_url}
              </a>
            ) : (
              "-"
            )}
          </Field>
          <Field label="主页">
            {project.homepage_url ? (
              <a
                className="text-blue-600 hover:underline"
                href={project.homepage_url}
                target="_blank"
                rel="noreferrer"
              >
                {project.homepage_url}
              </a>
            ) : (
              "-"
            )}
          </Field>
        </div>
      </div>

      {hasGithub && (
        <Section title="GitHub 指标">
          <div className="grid grid-cols-1 gap-x-8 md:grid-cols-2">
            <Field label="Stars">{formatCompact(project.stars)}</Field>
            <Field label="Forks">{formatCompact(project.forks)}</Field>
            <Field label="仓库创建">
              {project.github_created_at
                ? new Date(project.github_created_at).toLocaleDateString()
                : "-"}
            </Field>
            <Field label="仓库更新">
              {project.github_updated_at
                ? new Date(project.github_updated_at).toLocaleString()
                : "-"}
            </Field>
          </div>
        </Section>
      )}

      {hasHn && (
        <Section title="HackerNews 热度">
          <div className="mb-2 grid grid-cols-1 gap-x-8 md:grid-cols-2">
            <Field label="HN Points">{formatCompact(project.hn_points)}</Field>
            <Field label="HN 评论">{project.hn_comment_count ?? "-"}</Field>
          </div>
          {sources && sources.hackernews.length > 0 && (
            <div>
              <p className="mb-1 text-xs text-slate-500">相关 HN 讨论</p>
              {sources.hackernews.map((s) => (
                <HnStoryRow key={s.object_id} s={s} />
              ))}
            </div>
          )}
        </Section>
      )}

      <Section title="指标趋势（快照）">
        <MetricSparkline snapshots={snapshots ?? []} />
      </Section>

      {relatedTopic && relatedProjects.length > 0 && (
        <Section title={`相关项目 · ${relatedTopic}`}>
          <ul className="divide-y">
            {relatedProjects.map((p) => (
              <li key={p.id} className="py-2">
                <Link
                  to={{ pathname: `/projects/${p.id}`, search: returnSearch }}
                  className="font-medium text-blue-600 hover:underline"
                >
                  {p.full_name ?? p.name}
                </Link>
                <div className="flex items-center gap-2 text-xs text-slate-500">
                  <span>⭐{formatCompact(p.stars)}</span>
                  {p.hn_points != null && <span>🟧{formatCompact(p.hn_points)}</span>}
                  {p.language && <span>· {p.language}</span>}
                </div>
                {p.description && (
                  <p className="max-w-md truncate text-xs text-slate-500">{p.description}</p>
                )}
              </li>
            ))}
          </ul>
        </Section>
      )}
    </div>
  );
}
