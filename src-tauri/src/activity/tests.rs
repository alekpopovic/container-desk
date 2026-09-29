use super::*;
use crate::{contract_tests::scope, storage::tests::MemoryStorage};
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

fn managed() -> PolicyEngine {
    let mut policy = PolicyEngine::default();
    policy.register(scope()).unwrap();
    policy.set_access(&scope(), HostAccess::Manage).unwrap();
    policy
}
fn request(policy: &mut PolicyEngine) -> MutationRequest {
    let spec = MutationSpec {
        operation: MutationOperation::Restart,
        container_ids: vec![ContainerId("a".repeat(64))],
        timeout_seconds: 10,
    };
    let intent = policy
        .prepare(PrepareConfirmationRequest {
            scope: scope(),
            operation: ConfirmationOperation::Mutation(spec.clone()),
        })
        .unwrap();
    MutationRequest {
        scope: scope(),
        intent_id: intent.id,
        spec,
    }
}
fn memory() -> (Activities, MemoryStorage) {
    let disk = MemoryStorage::default();
    (Activities::load(Box::new(disk.clone())).unwrap(), disk)
}
#[test]
fn separate_confirmations_cannot_duplicate_an_inflight_host_operation_or_replay_after_loss() {
    let (activity, disk) = memory();
    let mut policy = managed();
    let first = request(&mut policy);
    let second = request(&mut policy);
    let replay = first.clone();
    let rejected = second.clone();
    let mut owner = activity.begin(&mut policy, first).unwrap();
    assert_eq!(
        activity.begin(&mut policy, second).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        activity
            .begin(&mut policy, replay.clone())
            .err()
            .unwrap()
            .code,
        ErrorCode::InvalidIntent
    );
    let plan = owner.dispatch().unwrap();
    assert_eq!(plan.args()[1], "restart");
    assert_eq!(
        owner.dispatch().err().unwrap().code,
        ErrorCode::InvalidIntent
    );
    // Interruption after dispatch leaves durable Unknown, without another transport invocation.
    drop(owner);
    assert_eq!(
        activity.records().unwrap()[0].outcome,
        ActivityOutcome::Unknown
    );
    let restarted = Activities::load(Box::new(disk)).unwrap();
    assert_eq!(
        restarted.records().unwrap()[0].outcome,
        ActivityOutcome::Unknown
    );
    assert_eq!(
        activity.begin(&mut policy, rejected).err().unwrap().code,
        ErrorCode::InvalidIntent
    );
    assert_eq!(
        activity.begin(&mut policy, replay).err().unwrap().code,
        ErrorCode::InvalidIntent
    );
    let next = request(&mut policy);
    let owner = activity.begin(&mut policy, next).unwrap();
    drop(owner);
    assert_eq!(
        activity.records().unwrap()[1].outcome,
        ActivityOutcome::NotDispatched
    );
}
#[test]
fn reconnect_cannot_release_a_still_owned_mutation_slot_and_read_only_bypass_is_denied() {
    let (activity, _) = memory();
    let mut policy = managed();
    let req = request(&mut policy);
    let owner = activity.begin(&mut policy, req).unwrap();
    policy.remove(&scope()).unwrap();
    policy.register(scope()).unwrap();
    let forged = MutationRequest {
        scope: scope(),
        intent_id: IntentId(format!("i_{}", "0".repeat(32))),
        spec: MutationSpec {
            operation: MutationOperation::Start,
            container_ids: vec![ContainerId("a".repeat(64))],
            timeout_seconds: 10,
        },
    };
    assert_eq!(
        activity.begin(&mut policy, forged).err().unwrap().code,
        ErrorCode::PermissionDenied
    );
    policy.set_access(&scope(), HostAccess::Manage).unwrap();
    let req = request(&mut policy);
    assert_eq!(
        activity.begin(&mut policy, req).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
    drop(owner);
    let req = request(&mut policy);
    assert!(activity.begin(&mut policy, req).is_ok());
}
#[test]
fn history_is_bounded_sanitized_and_reload_validates_untrusted_local_records() {
    let (activity, disk) = memory();
    let mut policy = managed();
    for n in 0..MAX_RECORDS + 5 {
        let req = request(&mut policy);
        let mut owner = activity.begin(&mut policy, req).unwrap();
        owner.dispatch().unwrap();
        owner
            .finish(if n % 2 == 0 {
                MutationOutcome::Succeeded
            } else {
                MutationOutcome::Failed
            })
            .unwrap();
    }
    let records = activity.records().unwrap();
    assert_eq!(records.len(), MAX_RECORDS);
    let bytes = disk.read(false).unwrap().unwrap();
    assert!(bytes.len() < MAX_BYTES);
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let keys = value["records"][0].as_object().unwrap();
    assert_eq!(keys.len(), 7);
    for forbidden in [
        "environment",
        "command",
        "terminal",
        "daemonId",
        "alias",
        "error",
    ] {
        assert!(!keys.contains_key(forbidden));
    }
    value["records"][0]["command"] = "secret untrusted text".into();
    disk.commit(&serde_json::to_vec(&value).unwrap(), None)
        .unwrap();
    assert_eq!(
        Activities::load(Box::new(disk)).err().unwrap().code,
        ErrorCode::StorageUnavailable
    );
}
struct Failing {
    disk: MemoryStorage,
    fail: Arc<AtomicBool>,
}
impl Storage for Failing {
    fn read(&self, previous: bool) -> io::Result<Option<Vec<u8>>> {
        self.disk.read(previous)
    }
    fn preserve_corrupt(&self) -> io::Result<()> {
        self.disk.preserve_corrupt()
    }
    fn commit(&self, bytes: &[u8], previous: Option<&[u8]>) -> io::Result<()> {
        if self.fail.load(Ordering::SeqCst) {
            Err(io::Error::other("injected write failure"))
        } else {
            self.disk.commit(bytes, previous)
        }
    }
}
#[test]
fn failed_durable_write_prevents_dispatch_and_uncertain_completion_remains_unknown() {
    for fail_at_dispatch in [true, false] {
        let disk = MemoryStorage::default();
        let fail = Arc::new(AtomicBool::new(false));
        let activity = Activities::load(Box::new(Failing {
            disk: disk.clone(),
            fail: fail.clone(),
        }))
        .unwrap();
        let mut policy = managed();
        let req = request(&mut policy);
        let mut owner = activity.begin(&mut policy, req).unwrap();
        if fail_at_dispatch {
            fail.store(true, Ordering::SeqCst);
            assert_eq!(
                owner.dispatch().err().unwrap().code,
                ErrorCode::StorageUnavailable
            );
            drop(owner);
        } else {
            owner.dispatch().unwrap();
            fail.store(true, Ordering::SeqCst);
            assert_eq!(
                owner.finish(MutationOutcome::Succeeded).unwrap_err().code,
                ErrorCode::StorageUnavailable
            );
        }
        fail.store(false, Ordering::SeqCst);
        let req = request(&mut policy);
        assert_eq!(
            activity.begin(&mut policy, req).err().unwrap().code,
            ErrorCode::StorageUnavailable
        );
        let restarted = Activities::load(Box::new(disk)).unwrap();
        assert_eq!(
            restarted.records().unwrap()[0].outcome,
            if fail_at_dispatch {
                ActivityOutcome::NotDispatched
            } else {
                ActivityOutcome::Unknown
            }
        );
    }
}
#[test]
fn native_file_history_has_private_permissions_and_recovers_unknown_without_replaying() {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!(
        "containerdesk-031-{}",
        crate::test_directory_suffix()
    ));
    let activity = Activities::open(&path).unwrap();
    let mut policy = managed();
    let req = request(&mut policy);
    let mut owner = activity.begin(&mut policy, req).unwrap();
    owner.dispatch().unwrap();
    drop(owner);
    drop(activity);
    assert_eq!(
        std::fs::metadata(path.join("activity/history.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(path.join("activity"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let loaded = Activities::open(&path).unwrap();
    assert_eq!(
        loaded.records().unwrap()[0].outcome,
        ActivityOutcome::Unknown
    );
    drop(loaded);
    std::fs::remove_dir_all(path).unwrap();
}

#[tokio::test]
async fn cancelled_dispatch_owner_retains_unknown_and_requires_a_fresh_confirmation() {
    use std::sync::atomic::AtomicUsize;
    let (activity, disk) = memory();
    let mut policy = managed();
    let req = request(&mut policy);
    let replay = req.clone();
    let mut operation = activity.begin(&mut policy, req).unwrap();
    let dispatches = Arc::new(AtomicUsize::new(0));
    let observed = dispatches.clone();
    let (sent, received) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _plan = operation.dispatch().unwrap();
        observed.fetch_add(1, Ordering::SeqCst);
        sent.send(()).unwrap();
        // A controlled transport stand-in blocks after accepting the command.
        std::future::pending::<()>().await;
        operation.finish(MutationOutcome::Succeeded).unwrap();
    });
    received.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(
        activity.records().unwrap()[0].outcome,
        ActivityOutcome::Unknown
    );
    let restored = Activities::load(Box::new(disk)).unwrap();
    assert_eq!(
        restored.records().unwrap()[0].outcome,
        ActivityOutcome::Unknown
    );
    assert_eq!(
        activity.begin(&mut policy, replay).err().unwrap().code,
        ErrorCode::InvalidIntent
    );
    assert_eq!(dispatches.load(Ordering::SeqCst), 1);
}
