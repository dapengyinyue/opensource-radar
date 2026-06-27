//! GitHub token 轮换器。MVP 单 token 也走同一路径，为多 token 扩量预留。

use std::sync::Mutex;

pub struct TokenRotator {
    tokens: Vec<String>,
    idx: Mutex<usize>,
}

impl TokenRotator {
    pub fn new(mut tokens: Vec<String>) -> Self {
        tokens.retain(|t| !t.is_empty());
        Self {
            tokens,
            idx: Mutex::new(0),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// 当前 token。无 token 时返回 None（匿名调用，受 GitHub 匿名限流）。
    pub fn current(&self) -> Option<String> {
        let tokens = &self.tokens;
        if tokens.is_empty() {
            return None;
        }
        let idx = *self.idx.lock().unwrap();
        Some(tokens[idx % tokens.len()].clone())
    }

    /// 轮换到下一个 token（当前 token 命中限流时调用）。
    pub fn rotate(&self) {
        if self.tokens.len() <= 1 {
            return;
        }
        let mut idx = self.idx.lock().unwrap();
        *idx = (*idx + 1) % self.tokens.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tokens_anonymous() {
        let r = TokenRotator::new(vec!["".into()]);
        assert!(r.is_empty());
        assert!(r.current().is_none());
    }

    #[test]
    fn rotates_between_tokens() {
        let r = TokenRotator::new(vec!["a".into(), "b".into()]);
        assert_eq!(r.current().as_deref(), Some("a"));
        r.rotate();
        assert_eq!(r.current().as_deref(), Some("b"));
        r.rotate();
        assert_eq!(r.current().as_deref(), Some("a"));
    }

    #[test]
    fn single_token_no_rotate() {
        let r = TokenRotator::new(vec!["a".into()]);
        r.rotate();
        assert_eq!(r.current().as_deref(), Some("a"));
    }
}
