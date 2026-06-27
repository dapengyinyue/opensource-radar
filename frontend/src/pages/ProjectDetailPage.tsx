import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";
import { getProject, listSnapshots } from "../api/client";
import SourceBadge from "../components/SourceBadge";
import MetricSparkline from "../components/MetricSparkline";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex gap-2 py-1">
      <span className="w-28 shrink-0 text-slate-500">{label}</span>
      <span className="flex-1">{children}</span>
    </div>
  );
}

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const numId = Number(id);

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

  if (isLoading) return <p className="text-slate-500">加载中…</p>;
  if (error) return <p className="text-red-600">加载失败：{(error as Error).message}</p>;
  if (!project) return <p className="text-slate-500">未找到项目。</p>;

  return (
    <div className="space-y-6">
      <div>
        <Link to="/" className="text-sm text-blue-600 hover:underline">
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
        {project.description && (
          <p className="mt-1 text-slate-600">{project.description}</p>
        )}

        <div className="mt-4 grid grid-cols-1 gap-x-8 md:grid-cols-2">
          <Field label="Stars">{project.stars ?? "-"}</Field>
          <Field label="HN Points">{project.hn_points ?? "-"}</Field>
          <Field label="Forks">{project.forks ?? "-"}</Field>
          <Field label="HN 评论">{project.hn_comment_count ?? "-"}</Field>
          <Field label="语言">{project.language ?? "-"}</Field>
          <Field label="Open Issues">{project.open_issues ?? "-"}</Field>
          <Field label="Topics">
            <div className="flex flex-wrap gap-1">
              {project.topics.map((t) => (
                <span key={t} className="rounded bg-slate-100 px-1.5 py-0.5 text-xs">
                  {t}
                </span>
              ))}
            </div>
          </Field>
          <Field label="活跃时间">
            {project.last_activity_at ? new Date(project.last_activity_at).toLocaleString() : "-"}
          </Field>
          <Field label="仓库">
            {project.repo_url ? (
              <a className="text-blue-600 hover:underline" href={project.repo_url} target="_blank" rel="noreferrer">
                {project.repo_url}
              </a>
            ) : (
              "-"
            )}
          </Field>
          <Field label="主页">
            {project.homepage_url ? (
              <a className="text-blue-600 hover:underline" href={project.homepage_url} target="_blank" rel="noreferrer">
                {project.homepage_url}
              </a>
            ) : (
              "-"
            )}
          </Field>
        </div>
      </div>

      <div className="rounded-lg border bg-white p-4">
        <h2 className="mb-2 font-medium">指标趋势（快照）</h2>
        <MetricSparkline snapshots={snapshots ?? []} />
      </div>
    </div>
  );
}
