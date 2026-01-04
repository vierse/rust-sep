import React from 'react';
import { eventReducer, type AppState, type Model } from './model';

/**
 * Sends a request to the backend to shorten a URL
 * @param url user-provided URL
 * @param signal abort signal to cancel the request
 * @returns Fully qualified shortened URL
 */
async function shortenUrl(url: string, signal: AbortSignal, name?: string): Promise<string> {

  const body = {
    url,
    ...(name && { name }),
  };

  const result = await fetch("/api/shorten", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal,
  });
  if (!result.ok) throw new Error(`Request error (${result.status})`);

  const data = (await result.json()) as { alias: string };
  if (!data.alias) throw new Error("Bad response: missing alias");

  return `${window.location.origin}/r/${data.alias}`;
}

/**
 * Initial model used by {@link useAppController}
 */
const initModel: Model = {
  state: { userUrl: "", userAlias: "", result: { kind: "none" } },
  effects: [],
}

/**
 * Event callbacks for the view layer.
 */
export type ViewEvents = {
  onUrlInput: (url: string) => void;
  onAliasInput: (url: string) => void;
  onSubmit: () => void;
  onRetry: () => void;
  onClear: () => void;
  onCopy: () => void;
  onCopyDismiss: () => void;
};

/**
 * App controller hook: ties reducer model with an effect runner.
 * @returns AppState to be rendered by UI and view callbacks
 */
export function useAppController(): {
  state: AppState,
  events: ViewEvents,
} {
  const [model, dispatch] = React.useReducer(eventReducer, initModel);

  // Abort controller for the currently running shorten request (if any)
  const abortRef = React.useRef<AbortController | null>(null);

  // True if the effect runner is processing an effect
  const runningRef = React.useRef(false);

  // Abort any running request when the component unmounts
  React.useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  // Effect runner: processes the effect queue and dispatches completion events
  React.useEffect(() => {
    if (model.effects.length === 0) return;

    // Snapshot the effects to be processed and remove them from the queue
    const queue = model.effects;
    dispatch({ kind: "effectsRun", count: queue.length });

    (async () => {
      try {
        for (const effect of queue) {
          switch (effect.kind) {

            case "shortenUrl": {

              // Return if the effect runner is already processing an effect
              if (runningRef.current) return;
              runningRef.current = true;

              // Abort any previous requests
              abortRef.current?.abort();
              const ac = new AbortController();
              abortRef.current = ac;

              // Set a request timeout
              const timeoutId = setTimeout(() => ac.abort(), 5_000);

              try {
                const shortUrl = await shortenUrl(effect.url, ac.signal, effect.name);
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
                runningRef.current = false;
              }
              break;
            }

            case "clipboardCopy": {
              try {
                await navigator.clipboard.writeText(effect.shortUrl);
              } catch (err) {
                // It's fine if we fail to copy to clipboard
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

  /**
   * Initialize view callbacks
   */
  const events: ViewEvents = React.useMemo(
    () => ({
      onUrlInput: (input: string) => dispatch({ kind: "setUrl", input }),
      onAliasInput: (input: string) => dispatch({ kind: "setAlias", input }),
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