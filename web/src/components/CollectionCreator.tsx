import { TextField, Box, Button, Flex, IconButton, Text } from "@radix-ui/themes";
import { Cross1Icon, PlusIcon, PaperPlaneIcon, Link2Icon, EraserIcon } from "@radix-ui/react-icons";

import React from "react";
import { postNoContent } from "../api";
import { clipboardCopy } from "../util";

type CreateCollectionRequest = {
  alias: string;
  urls: string[];
};

type State = "idle" | "ok" | "err";

export function CollectionCreator() {
  const [alias, setAlias] = React.useState("");
  const [urls, setUrls] = React.useState<string[]>([""]);
  const [state, setState] = React.useState<State>("idle");
  const [waiting, setWaiting] = React.useState(false);
  const [result, setResult] = React.useState("");

  const clearState = () => {
    setAlias("");
    setUrls([""]);
    setState("idle");
    setResult("");
  };

  const updateUrl = (index: number, value: string) => {
    setUrls((prev) => prev.map((u, i) => (i === index ? value : u)));
  };

  const addUrl = () => {
    setUrls((prev) => [...prev, ""]);
  };

  const removeUrl = (index: number) => {
    setUrls((prev) => prev.filter((_, i) => i !== index));
  };

  const submit = async () => {
    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);
    try {
      setWaiting(true);
      const filteredUrls = urls.filter((u) => u.trim() !== "");
      const body: CreateCollectionRequest = { alias, urls: filteredUrls };
      await postNoContent<CreateCollectionRequest>("/api/collection", body, ac.signal);
      const collectionUrl = `${window.location.origin}/?collection=${encodeURIComponent(alias)}`;
      setResult(collectionUrl);
      setState("ok");
    } catch (err) {
      setState("err");
      if (err instanceof DOMException && err.name === "AbortError") {
        console.log("Timeout error");
      } else {
        const errMsg = err instanceof Error ? err.message : "Unknown error";
        console.log(`Error: ${errMsg}`);
      }
    } finally {
      setWaiting(false);
      clearTimeout(timeoutId);
    }
  };

  const readOnly = waiting || state === "ok";
  const inputStatus = state === "idle" ? "" : state === "ok" ? "ok" : "err";

  return (
    <Flex direction="column" gap="3" style={{ width: "40rem", maxWidth: "90vw" }}>
      <Box data-status={inputStatus} className="inputField">
        <TextField.Root
          placeholder="Collection name"
          value={state === "ok" ? result : alias}
          readOnly={readOnly}
          onChange={(ev) => setAlias(ev.target.value)}
        >
          <TextField.Slot>
            <Link2Icon />
          </TextField.Slot>
        </TextField.Root>
      </Box>

      {state !== "ok" && (
        <>
          {urls.map((url, i) => (
            <Flex key={i} gap="2" align="center">
              <Text size="2" color="gray" style={{ minWidth: "1.5rem" }}>#{i}</Text>
              <Box style={{ flex: 1 }}>
                <TextField.Root
                  placeholder={`URL #${i}`}
                  value={url}
                  readOnly={readOnly}
                  onChange={(ev) => updateUrl(i, ev.target.value)}
                />
              </Box>
              {urls.length > 1 && (
                <IconButton
                  variant="ghost"
                  color="red"
                  disabled={readOnly}
                  onClick={() => removeUrl(i)}
                >
                  <Cross1Icon />
                </IconButton>
              )}
            </Flex>
          ))}

          <Button variant="soft" disabled={readOnly} onClick={addUrl}>
            <PlusIcon /> Add URL
          </Button>
        </>
      )}

      <Flex gap="2">
        <Button
          color={state === "ok" ? "indigo" : "green"}
          loading={waiting}
          onClick={async () => {
            if (state === "ok") {
              await clipboardCopy(result);
            } else {
              await submit();
            }
          }}
        >
          {state === "ok" ? <EraserIcon /> : <PaperPlaneIcon />}
        </Button>
        <Button color="red" disabled={waiting} onClick={clearState}>
          Clear
        </Button>
      </Flex>
    </Flex>
  );
}
