import React from 'react';
import { eventReducer, type AppState, type Model } from './model';

async function shortenUrl(url: string, signal: AbortSignal): Promise<string> {

  const result = await fetch("/api/shorten", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url }),
    signal,
  });
  if (!result.ok) throw new Error(`Request error (${result.status})`);

  const data = (await result.json()) as { shortUrl: string };
  if (!data.shortUrl) throw new Error("Bad response: missing shortUrl");

  return `${window.location.origin}/${data.shortUrl}`;
}

const initModel: Model = {
  state: { userInput: "", result: { kind: "none" } },
  effects: [],
}

export type ViewEvents = {
  onInput: (url: string) => void;
  onSubmit: () => void;
  onRetry: () => void;
  onClear: () => void;
  onCopy: () => void;
  onCopyDismiss: () => void;
};


export function useAppController(): {
  state: AppState,
  events: ViewEvents,
} {
  const [model, dispatch] = React.useReducer(eventReducer, initModel);

  const abortRef = React.useRef<AbortController | null>(null);
  const runningRef = React.useRef(false);

  React.useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  React.useEffect(() => {
    if (model.effects.length === 0) return;

    const queue = model.effects;
    dispatch({ kind: "effectsRun", count: queue.length });
    (async () => {
      try {
        for (const effect of queue) {
          switch (effect.kind) {

            case "shortenUrl": {

              if (runningRef.current) return;
              runningRef.current = true;

              abortRef.current?.abort();
              const ac = new AbortController();
              abortRef.current = ac;

              const timeoutId = setTimeout(() => ac.abort(), 5_000);

              try {
                const shortUrl = await shortenUrl(effect.url, ac.signal);
                dispatch({ kind: "requestOk", shortUrl });
              } catch (err) {
                if (err instanceof DOMException && err.name === "AbortError") {
                  dispatch({ kind: "requestErr", errMsg: "Request timed out" });
                } else {
                  const errMsg = err instanceof Error ? err.message : "Unknown error";
                  dispatch({ kind: "requestErr", errMsg });
                }
              } finally {
                clearTimeout(timeoutId);
              }
              break;
            }

            case "clipboardCopy": {
              try {
                await navigator.clipboard.writeText(effect.shortUrl);
              } catch (err) {
                const errMsg = err instanceof Error ? err.message : "Unknown error";
                console.error(`Failed to copy to clipboard: ${errMsg}`);
              } finally {
                dispatch({ kind: "copyDone" });
              }
              break;
            }
          }
        }
      } finally {
        runningRef.current = false;
      }
    })();
  }, [model.state, model.effects]);

  const events: ViewEvents = React.useMemo(
    () => ({
      onInput: (input: string) => dispatch({ kind: "setInput", input }),
      onSubmit: () => dispatch({ kind: "submit" }),
      onRetry: () => dispatch({ kind: "retry" }),
      onClear: () => dispatch({ kind: "clear" }),
      onCopy: () => dispatch({ kind: "copy" }),
      onCopyDismiss: () => dispatch({ kind: "copyDone" }),
    }),
    []
  );

  return { state: model.state, events };
}