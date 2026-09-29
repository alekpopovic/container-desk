import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ResourceLimits, ResourceLimitsReport } from "./ipc/generated.ts";
export const defaultLimits: ResourceLimits = Object.freeze({
  logLines: 20_000,
  logBytes: 8 * 1024 * 1024,
  statsHistory: 360,
  activeHosts: 1,
  concurrentJobs: 4,
});
let report: ResourceLimitsReport = {
  limits: defaultLimits,
  configurationIgnored: false,
};
export function validLimits(value: unknown): value is ResourceLimits {
  if (!value || typeof value !== "object") return false;
  const item = value as ResourceLimits;
  return Object.entries({
    logLines: [1000, 20_000],
    logBytes: [256 * 1024, 8 * 1024 * 1024],
    statsHistory: [60, 360],
    activeHosts: [0, 1],
    concurrentJobs: [2, 4],
  }).every(([key, bounds]) => {
    const number = item[key as keyof ResourceLimits];
    return (
      Number.isInteger(number) &&
      number >= (bounds[0] ?? 0) &&
      number <= (bounds[1] ?? 0)
    );
  });
}
export function resourceLimits() {
  return report.limits;
}
export function resourceLimitsReport() {
  return report;
}
/** Complete before mounting the application. Never change an active stream's bounds. */
export async function initializeResourceLimits() {
  if (!isTauri()) return;
  try {
    const value = await invoke<ResourceLimitsReport>("get_resource_limits");
    if (
      !value ||
      !validLimits(value.limits) ||
      typeof value.configurationIgnored !== "boolean"
    )
      throw new Error("Invalid resource limits");
    report = {
      limits: Object.freeze({ ...value.limits }),
      configurationIgnored: value.configurationIgnored,
    };
  } catch {
    report = { limits: defaultLimits, configurationIgnored: true };
  }
}
