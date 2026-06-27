-- 开源雷达 Phase-1 MVP schema

CREATE TYPE source_kind AS ENUM ('github', 'hackernews');

-- 归一主表：一个真实项目一行
CREATE TABLE projects (
    id                 BIGSERIAL PRIMARY KEY,
    dedup_key          TEXT NOT NULL UNIQUE,
    name               TEXT NOT NULL,
    full_name          TEXT,
    description        TEXT,
    repo_url           TEXT,
    homepage_url       TEXT,
    language           TEXT,
    topics             TEXT[] NOT NULL DEFAULT '{}',
    stars              BIGINT,
    forks              BIGINT,
    open_issues        BIGINT,
    hn_points          BIGINT,
    hn_comment_count   BIGINT,
    github_created_at  TIMESTAMPTZ,
    github_updated_at  TIMESTAMPTZ,
    last_activity_at   TIMESTAMPTZ,
    source_kinds       source_kind[] NOT NULL DEFAULT '{}',
    metadata           JSONB NOT NULL DEFAULT '{}'::jsonb,
    first_seen_at      TIMESTAMPTZ NOT NULL,
    last_collected_at  TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_proj_language      ON projects(language) WHERE language IS NOT NULL;
CREATE INDEX idx_proj_stars         ON projects(stars DESC NULLS LAST);
CREATE INDEX idx_proj_last_activity ON projects(last_activity_at DESC);
CREATE INDEX idx_proj_source_kinds  ON projects USING GIN(source_kinds);
CREATE INDEX idx_proj_topics        ON projects USING GIN(topics);
CREATE INDEX idx_proj_updated       ON projects(last_collected_at DESC);

-- GitHub 原始数据
CREATE TABLE raw_github_repos (
    id           BIGSERIAL PRIMARY KEY,
    project_id   BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    full_name    TEXT NOT NULL UNIQUE,
    node_id      TEXT,
    payload      JSONB NOT NULL,
    collected_at TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_rawgh_project ON raw_github_repos(project_id);

-- HackerNews 原始数据
CREATE TABLE raw_hn_stories (
    id            BIGSERIAL PRIMARY KEY,
    project_id    BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    object_id     TEXT NOT NULL UNIQUE,
    hn_url        TEXT NOT NULL,
    linked_url    TEXT,
    author        TEXT,
    points        BIGINT,
    comment_count BIGINT,
    posted_at     TIMESTAMPTZ,
    payload       JSONB NOT NULL,
    collected_at  TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_rawhn_project ON raw_hn_stories(project_id);

-- 趋势快照：每次采集写一行
CREATE TABLE project_snapshots (
    id          BIGSERIAL PRIMARY KEY,
    project_id  BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    stars       BIGINT,
    hn_points   BIGINT,
    captured_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_snap_proj_time ON project_snapshots(project_id, captured_at DESC);
