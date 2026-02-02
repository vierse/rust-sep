import React from "react";
import { Ev, eventReducer, initModel, type AppState, type Event } from "./model";
import { shorten } from "./api";

export type Dispatch = React.Dispatch<Event>;

export function useAppController(): {
  state: AppState, dispatch: Dispatch,
} {
  const [model, dispatch] = React.useReducer(eventReducer, initModel);
  const abortRef = React.useRef<AbortController | null>(null);
  const runningRef = React.useRef(false);

  // Abort inflight requests on unmount
  React.useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  React.useEffect(() => {
    if (model.effects.length === 0) return;

    const snapshot = model.effects;
    dispatch(Ev.updateEffects(snapshot.length));

    (async () => {
      try {
        for (const effect of snapshot) {
          if (runningRef.current) return;
          runningRef.current = true;

          switch (effect.kind) {
            case "copy": {
              try {
                await navigator.clipboard.writeText(effect.shortUrl);
              } catch (err) {
                // It's fine if we fail to copy to clipboard
                const errMsg = err instanceof Error ? err.message : "Unknown error";
                console.error(`Failed to copy to clipboard: ${errMsg}`);
              }
              break;
            }

            case "api/shorten": {
              abortRef.current?.abort();
              const ac = new AbortController();
              abortRef.current = ac;

              // 5s timeout
              const timeoutId = setTimeout(() => ac.abort(), 5_000);

              try {
                const shortUrl = await shorten(effect.userUrl, ac.signal, effect.urlName);
                dispatch(Ev.submitOk(shortUrl));
              } catch (err) {
                if (err instanceof DOMException && err.name === "AbortError") {
                  dispatch(Ev.submitErr("Request timed out"));
                } else {
                  const errMsg = err instanceof Error ? err.message : "Unknown error";
                  dispatch(Ev.submitErr(errMsg));
                }
              } finally {
                clearTimeout(timeoutId);
                runningRef.current = false;
              }
              break;
            }
          }
        }
      }
      finally {
        runningRef.current = false;
      }
    })();
  }, [model.state, model.effects]);

  return { state: model.state, dispatch };
}