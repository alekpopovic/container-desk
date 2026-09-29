import { useEffect } from "react";

export const primaryKey = /Mac/i.test(navigator.platform) ? "Meta" : "Control";

function visible(selector: string) {
  return Array.from(document.querySelectorAll<HTMLElement>(selector)).find(
    (element) =>
      element.getClientRects().length > 0 && !element.matches(":disabled"),
  );
}

export function useWorkspaceShortcuts() {
  useEffect(() => {
    let frame = 0;
    const keydown = (event: KeyboardEvent) => {
      const target = document.activeElement;
      if (
        event.defaultPrevented ||
        event.repeat ||
        event.isComposing ||
        event.altKey ||
        (primaryKey === "Meta"
          ? !event.metaKey || event.ctrlKey
          : !event.ctrlKey || event.metaKey) ||
        target?.closest(".terminal-panel, .xterm") ||
        document.querySelector("dialog[open]")
      )
        return;
      const key = event.key.toLowerCase();
      let selector: string;
      let destination =
        window.location.hash.replace(/^#\/?/, "") || "containers";
      let click = false;
      if (key === "f" && !event.shiftKey) {
        selector = '[data-shortcut="search"]';
        if (!visible(selector)) destination = "containers";
      } else if (key === "r" && !event.shiftKey) {
        selector = '[data-shortcut="refresh"]';
        if (!visible(selector)) return;
        click = true;
      } else if (key === "h" && event.shiftKey) {
        destination = "hosts";
        selector = '.host-cards button, [data-shortcut="new-host"]';
      } else if (key === "l" && event.shiftKey) {
        destination = "containers";
        selector = '[data-shortcut="logs"], [data-shortcut="search"]';
      } else return;
      event.preventDefault();
      window.location.hash = `/${destination}`;
      // Two frames let the existing route render before focus moves.
      frame = requestAnimationFrame(() => {
        frame = requestAnimationFrame(() => {
          if (window.location.hash !== `#/${destination}`) return;
          const element =
            key === "l"
              ? (visible('[data-shortcut="logs"]') ?? visible(selector))
              : visible(selector);
          if (click) element?.click();
          else element?.focus();
        });
      });
    };
    window.addEventListener("keydown", keydown);
    return () => {
      window.removeEventListener("keydown", keydown);
      cancelAnimationFrame(frame);
    };
  }, []);
}
