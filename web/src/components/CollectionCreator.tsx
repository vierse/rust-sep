import { TextField, Button, Flex, Box, IconButton, Text, Tooltip } from "@radix-ui/themes";
import { Link2Icon, PaperPlaneIcon, PlusIcon, Cross1Icon, IdCardIcon, LockClosedIcon, ResetIcon } from "@radix-ui/react-icons";

import React from "react";
import { postReq } from "../api";
import { useNotify } from "./NotifyProvider";

type CreateCollectionRequest = {
  alias?: string;
  password?: string;
  urls: string[];
};

type CreateCollectionResponse = {
  alias: string;
};

const URL_LIMIT = 10;

export function CollectionCreator() {
  // Notifications
  const { notifyErr, dismiss } = useNotify();

  // UI state
  const [urls, setUrls] = React.useState<string[]>([""]);
  const [waiting, setWaiting] = React.useState(false);

  // User state
  const [linkAlias, setLinkAlias] = React.useState<string | undefined>(undefined);
  const [linkPassword, setLinkPassword] = React.useState<string | undefined>(undefined);

  const clearState = () => {
    dismiss();

    setUrls([""]);

    setLinkAlias(undefined);
    setLinkPassword(undefined);
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
    const trimmedAlias = linkAlias?.trim();
    const trimmedPassword = linkPassword?.trim();
    const trimmedUrls = urls.map((u) => u.trim()).filter((u) => u.length > 0);

    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);

    dismiss();
    setWaiting(true);
    try {

      const body: CreateCollectionRequest = { urls: trimmedUrls };
      if (trimmedAlias) body.alias = trimmedAlias;
      if (trimmedPassword) body.password = trimmedPassword;

      const res = await postReq<CreateCollectionRequest, CreateCollectionResponse>(
        "/api/collection/create", body, ac.signal
      );

      const collectionUrl = `${window.location.origin}/collection/${encodeURIComponent(res.alias)}`;
      window.location.assign(collectionUrl);
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

  const canSubmit = urls.some((u) => u.trim().length > 0);
  const stateChanged = canSubmit || linkAlias || linkPassword || urls.length > 1;
  const limitReached = urls.length >= URL_LIMIT;
  return (
    <Flex direction="column" gap="3" style={{ width: "40rem" }}>
      <Flex direction="row" gap="3" style={{ width: "100%" }}>
        <Box style={{ flex: 1, minWidth: 0 }}>
          <Text size="2" color="gray" as="div" mb="1">
            Collection name (optional)
          </Text>
          <TextField.Root
            readOnly={waiting}
            value={linkAlias ?? ""}
            onChange={(e) => setLinkAlias(e.target.value)}
            size="2"
          >
            <TextField.Slot>
              <IdCardIcon />
            </TextField.Slot>
          </TextField.Root>
        </Box>

        <Box style={{ flex: 1, minWidth: 0 }}>
          <Text size="2" color="gray" as="div" mb="1">
            Password (optional)
          </Text>
          <TextField.Root
            placeholder="••••••••"
            type="password"
            readOnly={waiting}
            value={linkPassword ?? ""}
            onChange={(e) => setLinkPassword(e.target.value)}
            size="2"
          >
            <TextField.Slot>
              <LockClosedIcon />
            </TextField.Slot>
          </TextField.Root>
        </Box>
      </Flex>

      <Flex justify="between">
        <Flex direction="row" gap="3" style={{ width: "100%" }}>
          <Button variant="soft" onClick={addUrl} disabled={waiting || limitReached}>
            <PlusIcon />Add URL
          </Button>
          <Button color="green" loading={waiting} disabled={!canSubmit} onClick={submit}>
            <PaperPlaneIcon />Submit
          </Button>
        </Flex>

        <Tooltip content="Reset">
          <IconButton color="red" disabled={!stateChanged || waiting} onClick={clearState} radius="full">
            <ResetIcon />
          </IconButton>
        </Tooltip>
      </Flex>

      {urls.map((url, i) => (
        <Flex key={i} gap="2" align="center">
          <Box style={{ flex: 1 }}>
            <TextField.Root
              placeholder={`URL ${i}`}
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

    </Flex>
  );
}
