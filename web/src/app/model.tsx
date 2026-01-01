/**
 * App state that is rendered by the UI
 */
export type AppState = {
  userInput: string;
  result: Result;
}

/**
 * App state + effects queue
 */
export type Model = {
  state: AppState;
  effects: Effect[];
}

/**
 * States for user URL input
 */
type Result =
  | { kind: "none" }
  | { kind: "waiting" }
  | { kind: "copying"; shortUrl: string }
  | { kind: "ok"; shortUrl: string }
  | { kind: "err"; errMsg: string };

/**
 * Events used by {@link eventReducer}
 */
type Event =
  | { kind: "setInput"; input: string }
  | { kind: "submit" }
  | { kind: "requestOk"; shortUrl: string }
  | { kind: "requestErr"; errMsg: string }
  | { kind: "retry" }
  | { kind: "copy" }
  | { kind: "copyDone" }
  | { kind: "clear" }
  | { kind: "effectsRun"; count: number };

/**
 * Effects produced by {@link eventReducer}
 */
type Effect =
  | { kind: "shortenUrl"; url: string }
  | { kind: "clipboardCopy"; shortUrl: string }

/**
 * Returns a new model with effects appended to the effect queue.
 * @param m Current model
 * @param effects Effects to append
 * @returns A new model with effects appended
 */
function enqueue(m: Model, ...effects: Effect[]): Model {
  return { ...m, effects: m.effects.concat(effects) };
}

/**
 * Returns a new model with its state replaced
 * @param m Current model
 * @param state New state
 * @returns A new model with updated state
 */
function withState(m: Model, state: AppState): Model {
  return { ...m, state };
}

/**
 * Reducer managing state transitions for user URL input
 * @param m Current model
 * @param ev Incoming event
 * @returns Resulting model
 */
export function eventReducer(m: Model, ev: Event): Model {
  const { state } = m;

  switch (ev.kind) {
    case "setInput": {
      return withState(m, { userInput: ev.input, result: state.result });
    }

    case "submit": {
      if (state.result.kind === "waiting") return m;

      const url = state.userInput.trim();
      if (!url) {
        return withState(m, {
          ...state,
          result: { kind: "err", errMsg: "Enter a URL." },
        });
      }

      const effect = withState(m, { ...state, result: { kind: "waiting" } });
      return enqueue(effect, { kind: "shortenUrl", url });
    }

    case "requestOk": {
      const effect = withState(m, {
        ...state,
        result: { kind: "copying", shortUrl: ev.shortUrl },
      });
      return enqueue(effect, { kind: "clipboardCopy", shortUrl: ev.shortUrl });
    }

    case "requestErr": {
      return withState(m, {
        ...state,
        result: { kind: "err", errMsg: ev.errMsg },
      });
    }

    case "retry": {
      if (state.result.kind !== "err") return m;

      const url = state.userInput.trim();
      if (!url) {
        return withState(m, {
          ...state,
          result: { kind: "err", errMsg: "Enter a URL." },
        });
      }

      const effect = withState(m, { ...state, result: { kind: "waiting" } });
      return enqueue(effect, { kind: "shortenUrl", url });
    }

    case "copy": {
      if (state.result.kind !== "ok") return m;

      const effect = withState(m, {
        ...state,
        result: { kind: "copying", shortUrl: state.result.shortUrl },
      });
      return enqueue(effect, { kind: "clipboardCopy", shortUrl: state.result.shortUrl });
    }

    case "copyDone": {
      if (state.result.kind !== "copying") return m;

      return withState(m, { ...state, result: { kind: "ok", shortUrl: state.result.shortUrl } });
    }

    case "clear": {
      return withState(m, { ...state, userInput: "", result: { kind: "none" } });
    }

    case "effectsRun": {
      if (ev.count <= 0) return m;

      return { ...m, effects: m.effects.slice(ev.count) };
    }
  }
}