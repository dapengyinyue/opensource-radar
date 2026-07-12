//! 日报生成：取近 24h stars 增量 TOP N，拼 markdown。

use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, TimeZone, Utc};
use sqlx::PgPool;

use storage::repo::project;

/// 一份日报。
pub struct DigestReport {
    pub title: String,
    pub markdown: String,
    pub count: usize,
}

/// 生成「上升最快」日报：取近 24h stars 增量 TOP N。
/// 无上升项目返回 `Ok(None)`（当天不发空报）。
pub async fn generate_digest(pool: &PgPool, top_n: i64) -> Result<Option<DigestReport>> {
    let projects = project::rising(pool, 24, top_n).await?;

    if projects.is_empty() {
        return Ok(None);
    }

    let count = projects.len();
    let now = Utc::now();
    let title = format!("开源雷达日报 · {}月{}日", now.month(), now.day());
    let mut md = String::from("## 上升最快 TOP ");
    md.push_str(&count.to_string());
    md.push_str("\n\n");
    for (i, p) in projects.iter().enumerate() {
        let n = i + 1;
        let name = p.full_name.as_deref().unwrap_or(&p.name);
        let link = p.repo_url.as_deref().unwrap_or("#");
        let stars = p.stars.map(fmt_count).unwrap_or_else(|| "-".into());
        let delta = p
            .star_delta
            .map(|d| format!(" 🔺+{}", fmt_count(d)))
            .unwrap_or_default();
        let hn = p
            .hn_points
            .map(|v| format!("🟧{}", fmt_count(v)))
            .unwrap_or_default();
        md.push_str(&format!("{n}. **[{name}]({link})** ⭐{stars}{delta}"));
        if !hn.is_empty() {
            md.push_str(&format!(" · {hn}"));
        }
        md.push('\n');
        if let Some(desc) = &p.description {
            if !desc.is_empty() {
                md.push_str(&format!("   {}\n", desc.replace('\n', " ")));
            }
        }
    }

    Ok(Some(DigestReport {
        title,
        markdown: md,
        count,
    }))
}

fn fmt_count(n: i64) -> String {
    if n >= 10_000 {
        format!("{:.1}w", n as f64 / 10_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// 计算到下个 `hour:00`（本地意义由传入 now 决定，这里用 UTC 时刻）的延迟。
/// 若 now 早于今天 hour:00，延迟到今天 hour:00；否则延迟到明天 hour:00。
pub fn next_digest_delay(now: DateTime<Utc>, hour: u32) -> Duration {
    let today_target = today_at_hour(now, hour);
    let target = if now < today_target {
        today_target
    } else {
        today_target + ChronoDuration::days(1)
    };
    let dur = target.signed_duration_since(now);
    dur.to_std().unwrap_or(Duration::from_secs(0))
}

fn today_at_hour(now: DateTime<Utc>, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), hour.min(23), 0, 0)
        .single()
        .unwrap_or(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn delay_before_target_hour_is_same_day() {
        // 08:00 -> 09:00 当天，延迟 1h
        let now = at(2026, 7, 11, 8, 0);
        assert_eq!(next_digest_delay(now, 9), Duration::from_secs(3600));
    }

    #[test]
    fn delay_after_target_hour_is_next_day() {
        // 10:00 -> 次日 09:00，延迟 23h
        let now = at(2026, 7, 11, 10, 0);
        assert_eq!(next_digest_delay(now, 9), Duration::from_secs(23 * 3600));
    }

    #[test]
    fn delay_at_exact_target_hour_is_next_day() {
        // 正好 09:00 -> 次日 09:00，延迟 24h
        let now = at(2026, 7, 11, 9, 0);
        assert_eq!(next_digest_delay(now, 9), Duration::from_secs(24 * 3600));
    }

    #[test]
    fn delay_hour_zero_boundary() {
        // hour=0：00:30 -> 次日 00:00，延迟 23.5h
        let now = at(2026, 7, 11, 0, 30);
        assert_eq!(
            next_digest_delay(now, 0),
            Duration::from_secs(23 * 3600 + 30 * 60)
        );
    }

    #[test]
    fn delay_crosses_month_boundary() {
        // 7月31日 10:00 -> 8月1日 09:00
        let now = at(2026, 7, 31, 10, 0);
        assert_eq!(next_digest_delay(now, 9), Duration::from_secs(23 * 3600));
    }
}
