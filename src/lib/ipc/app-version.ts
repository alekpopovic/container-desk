import { invoke } from "@tauri-apps/api/core";

import type { AppVersion } from "./generated.ts";
export type { AppVersion } from "./generated.ts";

export async function getAppVersion(): Promise<AppVersion> {
  const response = await invoke<unknown>("app_version");
  if (
    typeof response !== "object" ||
    response === null ||
    !("version" in response) ||
    typeof response.version !== "string" ||
    response.version.length === 0 ||
    response.version.length > 128
  ) {
    throw new Error("Invalid application version response");
  }
  return { version: response.version };
}
