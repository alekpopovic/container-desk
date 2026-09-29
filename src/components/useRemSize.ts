import { useLayoutEffect, useState } from "react";

/** Observe a real rem-sized box; resize also catches text-only font scaling. */
export function useRemSize() {
  const [size, setSize] = useState(14);
  useLayoutEffect(() => {
    const probe = document.createElement("span");
    probe.setAttribute("aria-hidden", "true");
    probe.style.cssText =
      "position:fixed;visibility:hidden;pointer-events:none;width:1rem;height:1rem";
    document.body.append(probe);
    const update = () => setSize(probe.getBoundingClientRect().height);
    const observer = new ResizeObserver(update);
    observer.observe(probe);
    update();
    return () => {
      observer.disconnect();
      probe.remove();
    };
  }, []);
  return size;
}
