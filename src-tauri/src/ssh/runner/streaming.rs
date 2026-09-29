//! Non-PTY streams with bounded partial records; the sink must not block or retain without bounds.
use super::*;
pub(crate) const LINE_BYTES: usize = 256 * 1024 + 31;
pub(crate) type LineSink = Arc<dyn Fn(Stream, Vec<u8>, bool) + Send + Sync>;
impl Runner {
    pub(crate) fn start_stream(
        &self,
        executable: &str,
        args: Vec<OsString>,
        sink: LineSink,
        resource: Box<dyn Send>,
        session: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<Job, RunError> {
        if args.len() > 64
            || args
                .iter()
                .any(|a| a.as_bytes().len() > 128 * 1024 || a.as_bytes().contains(&0))
        {
            return Err(RunError::InvalidInput);
        }
        let executable = validate_executable(executable).map_err(|_| RunError::Unavailable)?;
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| RunError::Busy)?;
        let (cancel, cancelled) = oneshot::channel();
        let (result, receive) = oneshot::channel();
        tokio::spawn(async move {
            let outcome = execute(executable, args, sink, cancelled, session).await;
            drop(resource);
            drop(permit);
            let _ = result.send(outcome);
        });
        Ok(Job {
            cancel: Some(cancel),
            result: receive,
        })
    }
}
async fn lines(
    mut input: impl AsyncRead + Unpin,
    stream: Stream,
    sink: LineSink,
) -> Result<(), RunError> {
    let mut buffer = [0; 8192];
    let mut line = Vec::new();
    let mut truncated = false;
    loop {
        let n = input.read(&mut buffer).await.map_err(|_| RunError::Io)?;
        if n == 0 {
            if !line.is_empty() || truncated {
                sink(stream, line, truncated);
            }
            return Ok(());
        }
        for segment in buffer[..n].split_inclusive(|b| *b == b'\n') {
            let ends = segment.last() == Some(&b'\n');
            let bytes = if ends {
                &segment[..segment.len() - 1]
            } else {
                segment
            };
            let take = bytes.len().min(LINE_BYTES - line.len());
            line.extend_from_slice(&bytes[..take]);
            truncated |= take < bytes.len();
            if ends {
                sink(stream, std::mem::take(&mut line), truncated);
                truncated = false;
            }
        }
        // Huge always-ready streams must still yield to cancellation and other host work.
        tokio::task::yield_now().await;
    }
}
async fn execute(
    executable: PathBuf,
    args: Vec<OsString>,
    sink: LineSink,
    mut cancel: oneshot::Receiver<()>,
    mut session: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<Captured, RunError> {
    if !matches!(cancel.try_recv(), Err(oneshot::error::TryRecvError::Empty))
        || session
            .as_ref()
            .is_some_and(|r| *r.borrow() || r.has_changed().is_err())
    {
        return Err(RunError::Cancelled);
    }
    let mut owned = OwnedChild(
        owned_command(executable, args)
            .spawn()
            .map_err(|_| RunError::Unavailable)?,
    );
    let stdout = owned.0.stdout.take().expect("piped stdout");
    let stderr = owned.0.stderr.take().expect("piped stderr");
    let outcome = tokio::select! {
        biased;
        _ = &mut cancel => Err(RunError::Cancelled),
        _ = async {
            if let Some(receiver) = &mut session {
                while !*receiver.borrow() { if receiver.changed().await.is_err() { break; } }
            } else { std::future::pending::<()>().await; }
        } => Err(RunError::Cancelled),
        result = async {
            tokio::try_join!(lines(stdout, Stream::Stdout, sink.clone()), lines(stderr, Stream::Stderr, sink))?;
            let status = owned.0.wait().await.map_err(|_| RunError::Io)?;
            Ok(Captured { status, stdout: Vec::new(), stderr: Vec::new() })
        } => result,
    };
    if outcome.is_err() {
        owned.stop_and_reap().await;
    }
    outcome
}
