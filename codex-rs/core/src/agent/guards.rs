use crate::agent::role::AgentRole;
use crate::error::CodexErr;
use crate::error::Result;
use codex_protocol::ThreadId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

/// Session-scoped guardrails for spawned agents.
///
/// The same `AgentControl` instance is shared across a parent thread and all of the sub-agents it
/// spawns, so these limits naturally apply to the whole agent tree for a given user session.
#[derive(Default)]
pub(crate) struct Guards {
    threads: Mutex<HashMap<ThreadId, SpawnedThreadRecord>>,
    total_count: AtomicUsize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpawnedThreadRecord {
    pub(crate) parent_id: Option<ThreadId>,
    pub(crate) last_task_message: String,
    pub(crate) agent_type: AgentRole,
    pub(crate) depth: usize,
}

impl Guards {
    pub(crate) fn reserve_spawn_slot(
        self: &Arc<Self>,
        max_threads: Option<usize>,
    ) -> Result<SpawnReservation> {
        if let Some(max_threads) = max_threads {
            if !self.try_increment_spawned(max_threads) {
                return Err(CodexErr::AgentLimitReached { max_threads });
            }
        } else {
            self.total_count.fetch_add(1, Ordering::AcqRel);
        }

        Ok(SpawnReservation {
            state: Arc::clone(self),
            active: true,
        })
    }

    pub(crate) fn release_spawned_thread(&self, thread_id: ThreadId) {
        let removed = {
            let mut threads = self
                .threads
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            threads.remove(&thread_id)
        };
        if removed.is_some() {
            self.total_count.fetch_sub(1, Ordering::AcqRel);
        }
    }

    pub(crate) fn update_last_task_message(&self, thread_id: ThreadId, message: String) {
        let mut threads = self
            .threads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(record) = threads.get_mut(&thread_id) {
            record.last_task_message = message;
        }
    }

    pub(crate) fn live_agents(&self) -> Vec<(ThreadId, SpawnedThreadRecord)> {
        self.threads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .map(|(thread_id, record)| (*thread_id, record.clone()))
            .collect()
    }

    fn register_spawned_thread(&self, thread_id: ThreadId, record: SpawnedThreadRecord) {
        let mut threads = self
            .threads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        threads.insert(thread_id, record);
    }

    fn try_increment_spawned(&self, max_threads: usize) -> bool {
        let mut current = self.total_count.load(Ordering::Acquire);
        loop {
            if current >= max_threads {
                return false;
            }
            match self.total_count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(updated) => current = updated,
            }
        }
    }
}

pub(crate) struct SpawnReservation {
    state: Arc<Guards>,
    active: bool,
}

impl SpawnReservation {
    pub(crate) fn commit(mut self, thread_id: ThreadId, record: SpawnedThreadRecord) {
        self.state.register_spawned_thread(thread_id, record);
        self.active = false;
    }
}

impl Drop for SpawnReservation {
    fn drop(&mut self) {
        if self.active {
            self.state.total_count.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn reservation_drop_releases_slot() {
        let guards = Arc::new(Guards::default());
        let reservation = guards.reserve_spawn_slot(Some(1)).expect("reserve slot");
        drop(reservation);

        let reservation = guards.reserve_spawn_slot(Some(1)).expect("slot released");
        drop(reservation);
    }

    #[test]
    fn commit_holds_slot_until_release() {
        let guards = Arc::new(Guards::default());
        let reservation = guards.reserve_spawn_slot(Some(1)).expect("reserve slot");
        let thread_id = ThreadId::new();
        reservation.commit(
            thread_id,
            SpawnedThreadRecord {
                parent_id: None,
                last_task_message: "task".to_string(),
                agent_type: AgentRole::Default,
                depth: 1,
            },
        );

        let err = match guards.reserve_spawn_slot(Some(1)) {
            Ok(_) => panic!("limit should be enforced"),
            Err(err) => err,
        };
        let CodexErr::AgentLimitReached { max_threads } = err else {
            panic!("expected CodexErr::AgentLimitReached");
        };
        assert_eq!(max_threads, 1);

        guards.release_spawned_thread(thread_id);
        let reservation = guards
            .reserve_spawn_slot(Some(1))
            .expect("slot released after thread removal");
        drop(reservation);
    }
}
