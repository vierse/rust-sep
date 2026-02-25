export async function clipboardCopy(text: string) {
  console.log("trying to copy");
  try {
    await navigator.clipboard.writeText(text);
  } catch (err) {
    const errMsg = err instanceof Error ? err.message : "Unknown error";
    console.error(`Clipboard error: ${errMsg}`);
  }
}

export function urlWithAlias(alias: string): string {
  return `${window.location.origin}/r/${alias}`;
}