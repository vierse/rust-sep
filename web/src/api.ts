/**
 * Sends a request to the backend to shorten a URL
 * @param userUrl user-provided URL
 * @param abort abort signal to cancel the request
 * @param urlName optional URL name
 * @returns Fully qualified shortened URL
 */
export async function shorten(userUrl: string, abort: AbortSignal, urlName?: string): Promise<string> {

    const body = {
        url: userUrl,
        ...(urlName && { name: urlName }),
    };

    const result = await fetch("/api/shorten", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        signal: abort,
    });
    if (!result.ok) throw new Error(`Request error (${result.status})`);

    const data = (await result.json()) as { alias: string };
    if (!data.alias) throw new Error("Bad response: missing alias");

    return `${window.location.origin}/r/${data.alias}`;
}