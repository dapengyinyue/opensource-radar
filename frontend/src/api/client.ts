import type {
  Facet,
  ListResponse,
  Project,
  ProjectSources,
  Snapshot,
  SourceStatus,
  Sort,
  Since,
} from "./types";

const BASE = "/api/v1";

async function getJSON<T>(path: string): Promise<T> {
  const resp = await fetch(`${BASE}${path}`);
  if (!resp.ok) {
    const txt = await resp.text();
    throw new Error(`${resp.status}: ${txt}`);
  }
  return resp.json() as Promise<T>;
}

export interface ListParams {
  page?: number;
  per_page?: number;
  language?: string;
  topic?: string;
  source?: string;
  q?: string;
  sort?: Sort;
  since?: Since;
}

export function listProjects(params: ListParams): Promise<ListResponse> {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== "" && v !== null) q.set(k, String(v));
  }
  return getJSON<ListResponse>(`/projects?${q.toString()}`);
}

export function getProject(id: number): Promise<Project> {
  return getJSON<Project>(`/projects/${id}`);
}

export function listSnapshots(id: number): Promise<Snapshot[]> {
  return getJSON<Snapshot[]>(`/projects/${id}/snapshots`);
}

export function listLanguages(): Promise<Facet[]> {
  return getJSON<Facet[]>(`/languages`);
}

export function listTopics(): Promise<Facet[]> {
  return getJSON<Facet[]>(`/topics`);
}

export function getSourcesStatus(): Promise<SourceStatus[]> {
  return getJSON<SourceStatus[]>(`/sources/status`);
}

export function getProjectSources(id: number): Promise<ProjectSources> {
  return getJSON<ProjectSources>(`/projects/${id}/sources`);
}
