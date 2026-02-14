import { Cross1Icon } from "@radix-ui/react-icons";
import { Theme } from "@radix-ui/themes";
import { Toast, Portal } from "radix-ui";

import React from "react";

export const NotifyVariant = {
  Ok: "ok",
  Error: "err",
  Info: "info",
} as const;

export type NotifyVariant = (typeof NotifyVariant)[keyof typeof NotifyVariant];

type NotifyMsg = {
  title: string;
  description?: string;
  variant?: NotifyVariant;
  durationMs?: number;
};

type NotifyContextValue = {
  notify: (msg: NotifyMsg) => void;
  notifyOk: (msg: string) => void;
  notifyErr: (msg: string, errMsg?: string) => void;
  notifyShort: (msg: string) => void;
  dismiss: () => void;
};

const NotifyContext = React.createContext<NotifyContextValue | null>(null);

export function useNotify() {
  const ctx = React.useContext(NotifyContext);
  if (!ctx) throw new Error("useToast outside ToastProvider");
  return ctx;
}

export function NotifyProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = React.useState(false);
  const [msg, setMsg] = React.useState<NotifyMsg | null>(null);
  const timerRef = React.useRef(0);

  React.useEffect(() => {
    return () => {
      if (timerRef.current) window.clearTimeout(timerRef.current);
    };
  }, []);

  const notify = React.useCallback((msg: NotifyMsg) => {
    if (timerRef.current) window.clearTimeout(timerRef.current);

    if (open) {
      setOpen(false);
      timerRef.current = window.setTimeout(() => {
        setMsg(msg);
        setOpen(true);
      }, 100); // delay a little after closing current notification
      return;
    }

    setMsg(msg);
    setOpen(true);
  }, [open]);

  const notifyOk = React.useCallback((msg: string) => {
    notify({ title: msg, variant: NotifyVariant.Ok })
  }, [notify]);

  const notifyErr = React.useCallback((msg: string, errMsg?: string) => {
    notify({ title: msg, description: errMsg, variant: NotifyVariant.Error, durationMs: 10_000 })
  }, [notify]);

  const notifyShort = React.useCallback((msg: string) => {
    notify({ title: msg, variant: NotifyVariant.Info })
  }, [notify]);

  const dismiss = React.useCallback(() => {
    setOpen(false);
  }, []);

  return (
    <NotifyContext.Provider value={{ notify, notifyOk, notifyErr, notifyShort, dismiss }}>
      <Toast.Provider swipeDirection="right">
        {children}
        <Portal.Root>
          <Theme appearance="dark">
            <Toast.Root
              className="ToastRoot"
              data-variant={msg?.variant ?? NotifyVariant.Info}
              open={open}
              onOpenChange={setOpen}
              duration={msg?.durationMs ?? 3_000} // 3s default duration
            >
              <Toast.Title className="ToastTitle">
                {msg?.title}
              </Toast.Title>

              <Toast.Close asChild>
                <button className="ToastClose"><Cross1Icon /></button>
              </Toast.Close>

              {msg?.description ? (
                <Toast.Description className="ToastDescription">
                  {msg.description}
                </Toast.Description>
              ) : null}

            </Toast.Root>
            <Toast.Viewport className="ToastViewport" />
          </Theme>
        </Portal.Root>
      </Toast.Provider>
    </NotifyContext.Provider>
  );
}