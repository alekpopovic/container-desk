import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { getAppVersion } from "../lib/ipc/app-version";

type VersionState =
  | { status: "loading" }
  | { status: "ready"; version: string }
  | { status: "unavailable" }
  | { status: "error" };

function VersionRequest({ onRetry }: { onRetry: () => void }) {
  const [version, setVersion] = useState<VersionState>({ status: "loading" });

  useEffect(() => {
    let active = true;
    if (!isTauri()) {
      setVersion({ status: "unavailable" });
      return;
    }
    setVersion({ status: "loading" });
    void getAppVersion().then(
      (result) => {
        if (active) setVersion({ status: "ready", version: result.version });
      },
      () => {
        if (active) setVersion({ status: "error" });
      },
    );
    return () => {
      active = false;
    };
  }, []);

  return (
    <>
      <p role="status" aria-live="polite">
        {version.status === "loading" && "Loading application version…"}
        {version.status === "ready" && `Version ${version.version}`}
        {version.status === "unavailable" &&
          "Desktop version unavailable in browser."}
        {version.status === "error" &&
          "Application version could not be loaded."}
      </p>
      {version.status === "error" && (
        <button type="button" onClick={onRetry} className="button button-small">
          Try again
        </button>
      )}
    </>
  );
}

export function VersionInfo() {
  const [attempt, setAttempt] = useState(0);
  return (
    <VersionRequest
      key={attempt}
      onRetry={() => setAttempt((value) => value + 1)}
    />
  );
}
