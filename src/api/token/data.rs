use serde::{Deserialize, Serialize};

use crate::domain::LinkId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenEntry(LinkId, i64);

impl TokenEntry {
    pub fn id(self) -> LinkId {
        self.0
    }
    pub fn exp(self) -> i64 {
        self.1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenData<const TTL_SECS: i64, const CAP: usize>(Vec<TokenEntry>);

impl<const TTL_SECS: i64, const CAP: usize> TokenData<TTL_SECS, CAP> {
    pub fn update(&mut self, id: i64, now_s: i64) {
        // prune expired
        self.0.retain(|e| e.exp() > now_s);

        let entry = TokenEntry(id, now_s + TTL_SECS);

        // refresh existing
        if let Some(existing) = self.0.iter_mut().find(|e| e.id() == id) {
            *existing = entry;
            return;
        }

        // evict oldest to expire
        if self.0.len() >= CAP {
            if let Some((idx, _)) = self.0.iter().enumerate().min_by_key(|(_, e)| e.exp()) {
                self.0.swap_remove(idx);
            }
        }

        self.0.push(entry);
    }

    pub fn iter(&self) -> impl Iterator<Item = &TokenEntry> {
        self.0.iter()
    }
}
