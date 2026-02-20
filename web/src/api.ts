function containsJson(res: Response) {
  const ct = res.headers.get("content-type") ?? "";
  return ct.includes("application/json");
}

export async function postEmpty<ResponseType = void>(
  path: string,
  signal?: AbortSignal,
): Promise<ResponseType> {
  const res = await fetch(path, {
    method: "POST",
    headers: { Accept: "application/json" },
    ...(signal ? { signal } : {}),
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;
    try {
      const err = await res.json();
      if (typeof err === "string") reason = err;
    } catch {
      // ignore
    }
    throw new Error(reason);
  }

  if (containsJson(res)) return (await res.json()) as ResponseType;
  return undefined as ResponseType;
}

export async function postReq<RequestType, ResponseType = void>(
  path: string,
  body: RequestType,
  signal?: AbortSignal
): Promise<ResponseType> {
  const res = await fetch(path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify(body),
    ...(signal ? { signal } : {}),
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;
    try {
      const err = await res.json();
      if (typeof err === "string") reason = err;
    } catch {
      // ignore
    }
    throw new Error(reason);
  }

  if (containsJson(res)) {
    return (await res.json()) as ResponseType;
  }

  return undefined as ResponseType;
}

export async function getReq<ResponseType>(
  path: string,
  signal?: AbortSignal
): Promise<ResponseType> {
  const res = await fetch(path, {
    method: "GET",
    headers: {
      "Accept": "application/json"
    },
    ...(signal ? { signal } : {}),
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;

    try {
      const err = await res.json();
      if (typeof err === "string") reason = err;
    } catch {
      // ignore
    }

    throw new Error(reason);
  }

  return (await res.json()) as ResponseType;
}

export async function postNoContent<RequestType>(
  path: string,
  body: RequestType,
  signal?: AbortSignal
): Promise<void> {
  const res = await fetch(path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Accept": "application/json",
    },
    body: JSON.stringify(body),
    ...(signal ? { signal } : {}),
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;

    try {
      const err = (await res.json()) as { reason?: string };
      if (err?.reason) reason = err.reason;
    } catch {
      // ignore
    }

    throw new Error(reason);
  }
}

export async function deleteReq(path: string, signal?: AbortSignal): Promise<void> {
  const res = await fetch(path, {
    method: "DELETE",
    headers: { Accept: "application/json" },
    ...(signal ? { signal } : {}),
  });

  if (!res.ok) {
    let reason = `Request error (${res.status})`;

    try {
      const err = await res.json();
      if (typeof err === "string") reason = err;
    } catch {
      // ignore
    }

    throw new Error(reason);
  }
}