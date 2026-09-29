//! Native host admission is independent of renderer scheduling; guards release on every exit.
use crate::domain::{AppError, ErrorCode, HostId};
use std::{collections::HashMap, sync::Mutex};
pub(crate) struct Slots {
    active: Mutex<HashMap<HostId, usize>>,
    limit: usize,
}
impl Default for Slots {
    fn default() -> Self {
        Self::new(1)
    }
}
pub(crate) struct Slot<'a> {
    owner: &'a Slots,
    id: HostId,
}
impl Slots {
    pub fn new(limit: usize) -> Self {
        Self {
            active: Default::default(),
            limit,
        }
    }
    pub fn acquire(&self, id: &HostId) -> Result<Slot<'_>, AppError> {
        id.validate()?;
        let mut active = self
            .active
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        if (!active.contains_key(id) && active.len() >= 3)
            || active.get(id).copied().unwrap_or(0) >= self.limit
        {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        *active.entry(id.clone()).or_default() += 1;
        Ok(Slot {
            owner: self,
            id: id.clone(),
        })
    }
    #[cfg(test)]
    pub fn count(&self, id: &HostId) -> usize {
        self.active.lock().unwrap().get(id).copied().unwrap_or(0)
    }
}
impl Drop for Slot<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.owner.active.lock()
            && let Some(count) = active.get_mut(&self.id)
        {
            *count -= 1;
            if *count == 0 {
                active.remove(&self.id);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_hosts_are_bounded_and_guards_restore_admission() {
        let slots = Slots::new(2);
        let host = HostId(format!("h_{}", "a".repeat(32)));
        let first = slots.acquire(&host).unwrap();
        let second = slots.acquire(&host).unwrap();
        assert!(slots.acquire(&host).is_err());
        assert_eq!(slots.count(&host), 2);
        let other = HostId(format!("h_{}", "b".repeat(32)));
        let _other = slots.acquire(&other).unwrap();
        drop(first);
        assert_eq!(slots.count(&host), 1);
        drop(second);
        assert_eq!(slots.count(&host), 0);
        assert!(slots.acquire(&host).is_ok());
    }
}
