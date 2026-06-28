import { useEffect, useState } from "react";
import { Routes, Route, Link, useNavigate, useSearchParams } from "react-router-dom";
import HomePage from "./pages/HomePage";
import ProjectDetailPage from "./pages/ProjectDetailPage";

function SearchBox() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [value, setValue] = useState(params.get("q") ?? "");

  // URL 变化时回填输入框（如清空筛选、点 logo）
  useEffect(() => {
    setValue(params.get("q") ?? "");
  }, [params]);

  // 防抖：输入停顿 350ms 后跳转
  useEffect(() => {
    const q = value.trim();
    if (q === (params.get("q") ?? "")) return;
    const t = setTimeout(() => {
      const p = new URLSearchParams();
      if (q) p.set("q", q);
      navigate(`/?${p.toString()}`);
    }, 350);
    return () => clearTimeout(t);
  }, [value]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <input
      type="search"
      value={value}
      onChange={(e) => setValue(e.target.value)}
      placeholder="搜索项目名 / 描述…"
      className="ml-auto w-56 rounded border px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500"
      aria-label="搜索项目"
    />
  );
}

export default function App() {
  return (
    <div className="min-h-screen">
      <header className="border-b bg-white">
        <div className="mx-auto max-w-6xl px-4 py-3 flex items-center gap-2">
          <Link to="/" className="text-lg font-semibold">
            🛰️ 开源雷达
          </Link>
          <span className="text-sm text-slate-500">Open Source Radar</span>
          <SearchBox />
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-6">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/projects/:id" element={<ProjectDetailPage />} />
        </Routes>
      </main>
    </div>
  );
}
