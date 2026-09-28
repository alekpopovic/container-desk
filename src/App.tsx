import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { getAppVersion } from "./lib/ipc/app-version";

type VersionState =
  | { status: "loading" }
  | { status: "ready"; version: string }
  | { status: "unavailable" }
  | { status: "error" };

function VersionInfo({ onRetry }: { onRetry: () => void }) {
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
          "Open the desktop app to load its version."}
        {version.status === "error" &&
          "Application version could not be loaded."}
      </p>
      {version.status === "error" && (
        <button
          type="button"
          onClick={onRetry}
          className="mt-3 rounded-md border border-slate-400 px-3 py-2 hover:bg-slate-200 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-teal-600 dark:hover:bg-slate-800"
        >
          Try again
        </button>
      )}
    </>
  );
}

export default function App() {
  const [attempt, setAttempt] = useState(0);
  return (
    <main className="mx-auto flex min-h-screen max-w-3xl flex-col justify-center px-8 py-12">
      <div
        className="mb-8 grid size-14 place-items-center rounded-2xl bg-teal-700 text-xl font-semibold text-white"
        aria-hidden="true"
      >
        CD
      </div>
      <p className="mb-3 text-sm font-medium tracking-wide text-teal-700 dark:text-teal-400">
        ContainerDesk
      </p>
      <h1 className="text-4xl font-semibold tracking-tight">
        Welcome to your workspace.
      </h1>
      <p className="mt-5 max-w-lg text-lg leading-relaxed text-slate-600 dark:text-slate-400">
        A desktop home for your remote Docker hosts.
      </p>
      <footer className="mt-12 border-t border-slate-300 pt-5 text-sm text-slate-600 dark:border-slate-700 dark:text-slate-400">
        <VersionInfo
          key={attempt}
          onRetry={() => setAttempt((value) => value + 1)}
        />
      </footer>
    </main>
  );
}
