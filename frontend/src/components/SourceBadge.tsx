export default function SourceBadge({ source }: { source: string }) {
  const styles: Record<string, string> = {
    github: "bg-slate-800 text-white",
    hackernews: "bg-orange-600 text-white",
  };
  return (
    <span
      className={`inline-block rounded px-1.5 py-0.5 text-xs font-medium ${
        styles[source] ?? "bg-slate-300 text-slate-800"
      }`}
    >
      {source}
    </span>
  );
}
