export type AppState = {
  userInput: string;
  result: Result;
}

export type Model = {
  state: AppState;
  effects: Effect[];
}

type Result =
  | { kind: "none" }
  | { kind: "waiting" }
  | { kind: "copying"; shortUrl: string }
  | { kind: "ok"; shortUrl: string }
  | { kind: "err"; errMsg: string };

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

type Effect =
  | { kind: "shortenUrl"; url: string }
  | { kind: "clipboardCopy"; shortUrl: string }

function enqueue(m: Model, ...effects: Effect[]): Model {
  return { ...m, effects: m.effects.concat(effects) };
}

function withState(m: Model, state: AppState): Model {
  return { ...m, state };
}

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