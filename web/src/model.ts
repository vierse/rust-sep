
type Result =
  | { kind: "none" }
  | { kind: "inflight" }
  | { kind: "ok"; shortUrl: string }
  | { kind: "err"; errMsg: string };

type User = {
  username: string
}

export const Ev = {
  updateEffects: (runCount: number) => ({ kind: "updateEffects", runCount } as const),
  copy: () => ({ kind: "copy" } as const),
  submit: (userUrl: string, urlName?: string) => ({ kind: "submit", userUrl, urlName } as const),
  submitOk: (shortUrl: string) => ({ kind: "submitOk", shortUrl } as const),
  submitErr: (errMsg: string) => ({ kind: "submitErr", errMsg } as const),
  clear: () => ({ kind: "clear" } as const),
};

export type Event = ReturnType<(typeof Ev)[keyof typeof Ev]>;

export type Effect =
  | { kind: "copy"; shortUrl: string }
  | { kind: "api/shorten"; userUrl: string; urlName?: string; }

export type AppState = {
  result: Result;
  user: User | undefined;
}

export type Model = {
  state: AppState;
  effects: Effect[];
}

export const initModel: Model = {
  state: { result: { kind: "none" }, user: undefined },
  effects: [],
}

function enqueue(m: Model, ...effects: Effect[]): Model {
  return { ...m, effects: m.effects.concat(effects) };
}

function withState(m: Model, state: AppState): Model {
  return { ...m, state };
}

export function eventReducer(m: Model, ev: Event): Model {
  const { state } = m;

  switch (ev.kind) {
    case "updateEffects": {
      return { ...m, effects: m.effects.slice(ev.runCount) };
    }
    case "copy": {
      if (state.result.kind !== "ok") return m;
      return enqueue(m, { kind: "copy", shortUrl: state.result.shortUrl });
    }
    case "submit": {
      if (state.result.kind !== "none" && state.result.kind !== "err") return m;
      const next = withState(m, { ...state, result: { kind: "inflight" } });
      return enqueue(next, { kind: "api/shorten", userUrl: ev.userUrl, urlName: ev.urlName });
    }
    case "submitOk": {
      const next = withState(m, { ...state, result: { kind: "ok", shortUrl: ev.shortUrl } });
      return enqueue(next, { kind: "copy", shortUrl: ev.shortUrl });
    }
    case "submitErr": {
      return withState(m, { ...state, result: { kind: "err", errMsg: ev.errMsg } });
    }
    case "clear": {
      return withState(m, { ...state, result: { kind: "none" } });
    }
  }
}