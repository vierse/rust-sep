import { TextField, Button, Flex, Box, IconButton } from "@radix-ui/themes";
import { Link2Icon, PaperPlaneIcon, EraserIcon, PlusIcon, Cross1Icon, ClipboardIcon } from "@radix-ui/react-icons";

import React from "react";
import { postNoContent } from "../api";
import { clipboardCopy } from "../util";
import { useNotify } from "./NotifyProvider";

type CreateCollectionRequest = {
  alias: string;
  urls: string[];
};

export function CollectionCreator() {
  const { notifyOk, notifyErr, dismiss } = useNotify();

  const [alias, setAlias] = React.useState("");
  const [urls, setUrls] = React.useState<string[]>([""]);
  const [waiting, setWaiting] = React.useState(false);
  const [result, setResult] = React.useState("");

  const clearState = () => {
    setAlias("");
    setUrls([""]);
    setResult("");
    dismiss();
  };

  const setUrl = (index: number, value: string) => {
    setUrls((prev) => prev.map((u, i) => (i === index ? value : u)));
  };

  const addUrl = () => {
    setUrls((prev) => [...prev, ""]);
  };

  const removeUrl = (index: number) => {
    setUrls((prev) => prev.filter((_, i) => i !== index));
  };

  const submit = async () => {
    const trimmedAlias = alias.trim();
    const trimmedUrls = urls.map((u) => u.trim()).filter((u) => u.length > 0);

    if (!trimmedAlias) {
      notifyErr("Missing alias", "Enter an alias for your collection");
      return;
    }
    if (trimmedUrls.length === 0) {
      notifyErr("No URLs", "Add at least one URL to your collection");
      return;
    }

    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);
    try {
      dismiss();
      setWaiting(true);

      const body: CreateCollectionRequest = { alias: trimmedAlias, urls: trimmedUrls };
      await postNoContent("/api/collection", body, ac.signal);

      const collectionUrl = `${window.location.origin}/collection/${encodeURIComponent(trimmedAlias)}`;
      await clipboardCopy(collectionUrl);
      setResult(collectionUrl);
      notifyOk("Collection created — link copied!");
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") {
        notifyErr("Server error", "Request timed out");
      } else {
        const errMsg = err instanceof Error ? err.message : "Unknown error";
        notifyErr("Could not create collection", errMsg);
      }
    } finally {
      setWaiting(false);
      clearTimeout(timeoutId);
    }
  };

  const canSubmit = alias.trim().length > 0 && urls.some((u) => u.trim().length > 0);

  if (result) {
    return (
      <Flex direction="column" gap="3" style={{ width: "40rem" }}>
        <Box data-status="ok" className="inputField">
          <TextField.Root value={result} readOnly style={{ width: "40rem" }}>
            <TextField.Slot><Link2Icon /></TextField.Slot>
          </TextField.Root>
        </Box>
        <Flex gap="2" justify="center">
          <Button color="indigo" onClick={async () => {
            await clipboardCopy(result);
            notifyOk("Copied to clipboard!");
          }}>
            <ClipboardIcon />
          </Button>
          <Button color="red" onClick={clearState}>
            <EraserIcon />
          </Button>
        </Flex>
      </Flex>
    );
  }

  return (
    <Flex direction="column" gap="3" style={{ width: "40rem" }}>
      <TextField.Root
        placeholder="Collection alias"
        value={alias}
        onChange={(e) => setAlias(e.target.value)}
      />

      {urls.map((url, i) => (
        <Flex key={i} gap="2" align="center">
          <Box style={{ flex: 1 }}>
            <TextField.Root
              placeholder={`URL ${i + 1}`}
              value={url}
              onChange={(e) => setUrl(i, e.target.value)}
            >
              <TextField.Slot><Link2Icon /></TextField.Slot>
            </TextField.Root>
          </Box>
          {urls.length > 1 && (
            <IconButton variant="ghost" color="red" onClick={() => removeUrl(i)}>
              <Cross1Icon />
            </IconButton>
          )}
        </Flex>
      ))}

      <Button variant="soft" onClick={addUrl}>
        <PlusIcon /> Add URL
      </Button>

      <Flex gap="2" justify="center">
        <Button color="green" loading={waiting} disabled={!canSubmit} onClick={submit}>
          <PaperPlaneIcon />
        </Button>
        <Button color="red" disabled={waiting} onClick={clearState}>
          <EraserIcon />
        </Button>
      </Flex>
    </Flex>
  );
}
