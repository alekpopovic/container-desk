import {
  useLayoutEffect,
  useRef,
  type ReactNode,
  type RefObject,
  type KeyboardEvent,
} from "react";

/** Remember the invoker before asynchronous preparation disables it. */
export function useConfirmationFocus() {
  const last = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    const remember = (event: FocusEvent) => {
      const element = event.target;
      if (
        element instanceof HTMLElement &&
        element !== document.body &&
        !element.closest("dialog")
      )
        last.current = element;
    };
    document.addEventListener("focusin", remember);
    return () => document.removeEventListener("focusin", remember);
  }, []);
  return last;
}

/** Native modal semantics, cancellation before dispatch, and trigger restoration. */
export function ConfirmationDialog({
  children,
  label,
  className,
  busy = false,
  onCancel,
  returnFocus,
}: {
  children: ReactNode;
  label: string;
  className: string;
  busy?: boolean;
  onCancel: () => void;
  returnFocus: RefObject<HTMLElement | null>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const trigger = returnFocus.current ?? document.activeElement;
    const element = dialog.current;
    element?.showModal();
    element?.querySelector<HTMLButtonElement>("[data-cancel]")?.focus();
    return () => {
      element?.close();
      queueMicrotask(() => {
        if (
          trigger instanceof HTMLElement &&
          trigger.isConnected &&
          !document.querySelector("dialog[open]")
        )
          trigger.focus();
      });
    };
  }, [returnFocus]);
  return (
    <dialog
      ref={dialog}
      aria-label={label}
      aria-busy={busy}
      className={`confirmation-dialog ${className}`}
      onKeyDown={trapDialogTab}
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onCancel();
      }}
    >
      {children}
    </dialog>
  );
}

export function trapDialogTab(event: KeyboardEvent<HTMLDialogElement>) {
  if (event.key !== "Tab") return;
  const controls = Array.from(
    event.currentTarget.querySelectorAll<HTMLElement>(
      'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]',
    ),
  ).filter((element) => element.getClientRects().length > 0);
  const first = controls[0],
    last = controls[controls.length - 1];
  if (!first) {
    event.preventDefault();
    return;
  }
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last?.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
