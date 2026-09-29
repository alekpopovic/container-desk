import { trapDialogTab } from "./ConfirmationDialog";
import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import {
  clearSupportData,
  IpcError,
  prepareSupportReport,
  saveSupportReport,
} from "../lib/ipc/client";
import type { SupportPreview } from "../lib/ipc/generated";
export function SupportDiagnostics({ demo = false }: { demo?: boolean }) {
  const native = isTauri() && !demo;
  const [preview, setPreview] = useState<SupportPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const active = useRef(false);
  const working = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  async function run(action: () => Promise<void>) {
    if (!native || working.current) return;
    working.current = true;
    setBusy(true);
    setMessage("");
    try {
      await action();
    } catch (error) {
      if (active.current)
        setMessage(
          error instanceof IpcError
            ? error.message
            : "Support data could not be updated.",
        );
    } finally {
      working.current = false;
      if (active.current) setBusy(false);
    }
  }
  return (
    <section
      className="support-panel"
      aria-labelledby="support-title"
      aria-busy={busy}
    >
      <h3 id="support-title">Support report and local history</h3>
      <p>
        The report includes app/platform versions, connection mode, error codes,
        counts and coarse timings. Host and path names use report-local
        pseudonyms. No user-supplied text, keys, SSH configuration, environment
        values, logs, terminal text or inspect payloads are included.
      </p>
      <p>
        Review before sharing: versions, operation outcomes and counts still
        describe your activity. The preview is kept in memory, limited to 64
        KiB, and valid for five minutes. Preparing a new report replaces the
        previous preview.
      </p>
      <button
        type="button"
        className="button"
        disabled={!native || busy}
        onClick={() =>
          void run(async () => {
            const value = await prepareSupportReport();
            if (active.current) setPreview(value);
          })
        }
      >
        Prepare support preview
      </button>
      {preview && (
        <>
          <textarea
            className="support-preview"
            aria-label="Support report preview"
            readOnly
            rows={16}
            value={preview.report}
          />
          <button
            type="button"
            className="button"
            disabled={!native || busy}
            onClick={() =>
              void run(async () => {
                const saved = await saveSupportReport(preview.id);
                if (active.current) {
                  setMessage(
                    saved
                      ? "Reviewed support report saved."
                      : "Save cancelled; no report written.",
                  );
                  if (saved) setPreview(null);
                }
              })
            }
          >
            Save reviewed report…
          </button>
        </>
      )}
      <p>
        Local activity retains at most 200 records or 512 KiB, dropping the
        oldest eligible entries. Reports include only the latest 20. Records
        have no age-based expiry and remain until replaced or cleared; this is
        not an audit archive.
      </p>
      <button
        type="button"
        className="button"
        disabled={!native || busy}
        onClick={() => {
          setMessage("");
          dialog.current?.showModal();
        }}
      >
        Clear local troubleshooting data…
      </button>
      <p>
        Clearing removes local activity history and the prepared report. Saved
        hosts, preferences, SSH files and files you exported remain. Finish
        active operations and close any save dialog first. New operations can
        create new history.
      </p>
      {!native && (
        <p>
          {demo
            ? "Support export and clearing are disabled in DEMO."
            : "Open the desktop app to manage support data."}
        </p>
      )}
      {message && <p role="status">{message}</p>}
      <dialog
        onKeyDown={trapDialogTab}
        ref={dialog}
        className="support-confirmation"
        aria-labelledby="clear-support-title"
        onCancel={(event) => {
          if (busy) event.preventDefault();
        }}
      >
        {message && <p role="status">{message}</p>}
        <h3 id="clear-support-title">Clear local troubleshooting data?</h3>
        <p>
          This deletes the local activity history, including unknown outcomes,
          and the prepared support report. It does not undo remote actions. SSH
          config, keys, known_hosts, saved hosts and exported files are
          retained.
        </p>
        <button
          type="button"
          className="button"
          disabled={busy}
          onClick={() => dialog.current?.close()}
        >
          Cancel
        </button>
        <button
          type="button"
          className="button"
          disabled={busy}
          onClick={() =>
            void run(async () => {
              await clearSupportData();
              if (active.current) {
                setPreview(null);
                setMessage("Local activity and prepared report cleared.");
                dialog.current?.close();
              }
            })
          }
        >
          Clear local history and preview
        </button>
      </dialog>
    </section>
  );
}
