type ApiErrorBody = { reason: string };

export async function postJson<RequestType, ResponseType>(
  path: string,
  body: RequestType,
  abort: AbortSignal
): Promise<ResponseType> {
  const res = await fetch(path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Accept": "application/json",
    },
    body: JSON.stringify(body),
    signal: abort,
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;

    try {
      const err = (await res.json()) as ApiErrorBody;
      reason = err.reason;
    } catch {
      // ignore
    }

    throw new Error(reason);
  }
  return (await res.json()) as ResponseType;
}

export async function getJson<ResponseType>(
  path: string,
  abort: AbortSignal
): Promise<ResponseType> {
  const res = await fetch(path, {
    method: "GET",
    headers: {
      "Accept": "application/json"
    },
    signal: abort,
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;

    try {
      const err = (await res.json()) as ApiErrorBody;
      reason = err.reason;
    } catch {
      // ignore
    }

    throw new Error(reason);
  }

  return (await res.json()) as ResponseType;
}