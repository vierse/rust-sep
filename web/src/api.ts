export async function postJson<RequestType, ResponseType>(
  path: string,
  body: RequestType,
  signal?: AbortSignal
): Promise<ResponseType> {
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
      const err = await res.json();
      if (typeof err === "string") reason = err;
    } catch {
      // ignore
    }

    throw new Error(reason);
  }
  return (await res.json()) as ResponseType;
}

export async function getJson<ResponseType>(
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