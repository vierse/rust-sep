export async function clipboardCopy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (err) {
    const errMsg = err instanceof Error ? err.message : "Unknown error";
    console.error(`Clipboard error: ${errMsg}`);
  }
}