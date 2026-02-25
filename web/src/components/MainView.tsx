import { Box, Flex, Button, TextField, Card, IconButton, Tooltip } from "@radix-ui/themes";
import { ClipboardIcon, DotsHorizontalIcon, Link2Icon, PaperPlaneIcon, PlusIcon, ReloadIcon, ResetIcon } from "@radix-ui/react-icons";

import React from "react";
import { useNotify } from "./NotifyProvider";
import { postEmpty, postReq } from "../api";
import { clipboardCopy, urlWithAlias } from "../util";

type ShortenRequest = {
  url: string;
  alias?: string;
  password?: string;
};

type ShortenResponse = {
  alias: string;
};

type State = "idle" | "ok" | "err";

export function MainView() {
  // Notifications
  const { notifyOk, notifyErr, notifyShort, dismiss } = useNotify();

  // UI state
  const [state, setState] = React.useState<State>("idle");
  const [showOptions, setShowOptions] = React.useState(false);
  const [waiting, setWaiting] = React.useState(false);
  const [result, setResult] = React.useState("");
  const [resultUrl, setResultUrl] = React.useState("");

  // User state
  const [userUrl, setUserUrl] = React.useState("");
  const [linkAlias, setLinkAlias] = React.useState<string | undefined>(undefined);
  const [linkPassword, setLinkPassword] = React.useState<string | undefined>(undefined);

  const toggleOptions = () => {
    setLinkAlias(undefined);
    setLinkPassword(undefined);
    setShowOptions(!showOptions);
  }

  const clearState = () => {
    dismiss();

    setState("idle");
    setShowOptions(false);
    setWaiting(false);
    setResult("");
    setResultUrl("");

    setUserUrl("");
    setLinkAlias(undefined);
    setLinkPassword(undefined);
  };

  const shortenUrl = async () => {
    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);
    try {
      dismiss();
      setWaiting(true);

      const body = { url: userUrl, alias: linkAlias || undefined, password: linkPassword || undefined } as ShortenRequest;
      const res = await postReq<ShortenRequest, ShortenResponse>("/api/shorten", body, ac.signal);
      setResult(res.alias);
      setResultUrl(urlWithAlias(res.alias));
      setState("ok");
      if (showOptions) toggleOptions();

      notifyOk("New link created");
    } catch (err) {
      setState("err");
      if (err instanceof DOMException && err.name === "AbortError") {
        notifyErr("Server error", "Request timed out");
        console.log("Timeout error");
      } else {
        const errMsg = err instanceof Error ? err.message : "Unknown error";
        notifyErr("Could not create a link", errMsg);
        console.log(`Error: ${errMsg}`);
      }
    } finally {
      setWaiting(false);
      clearTimeout(timeoutId);
    }
  };

  const createCollection = async () => {
    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);
    try {
      dismiss();
      setWaiting(true);

      const res = await postEmpty(`/api/collection/create/${encodeURIComponent(result)}`) as string;
      window.location.assign(res);
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") {
        notifyErr("Server error", "Request timed out");
        console.log("Timeout error");
      } else {
        const errMsg = err instanceof Error ? err.message : "Unknown error";
        notifyErr("Could not create a collection", errMsg);
        console.log(`Error: ${errMsg}`);
      }
    } finally {
      setWaiting(false);
      clearTimeout(timeoutId);
    }
  };

  const canSubmit = userUrl.trim().length > 0;
  const readOnly = waiting || state === "ok";
  return (
    <Box style={{ position: "relative" }}>
      <Flex align="center" gap="2">
        <Box data-status={state} className="inputField" style={{ width: "30rem" }}>

          {/* URL window */}
          <TextField.Root
            value={state === "ok" ? resultUrl : userUrl}
            readOnly={readOnly}
            onChange={(e) => setUserUrl(e.target.value)}
            placeholder="Paste a URL"
            size="3"
          >
            <TextField.Slot>
              <Link2Icon />
            </TextField.Slot>

            <TextField.Slot>
              <Tooltip content="Copy to clipboard">
                <IconButton
                  variant="ghost"
                  disabled={state !== "ok"}
                  onClick={() => {
                    clipboardCopy(resultUrl);
                    notifyShort("Copied to clipboard");
                  }}>
                  <ClipboardIcon />
                </IconButton>
              </Tooltip>
            </TextField.Slot>
          </TextField.Root>
        </Box>

        {/* main button */}
        {(() => {
          switch (state) {
            case "idle":
              return (
                <Button
                  color="green"
                  loading={waiting}
                  disabled={!canSubmit}
                  onClick={shortenUrl}
                >
                  <PaperPlaneIcon />Shorten
                </Button>
              );

            case "err":
              return (
                <Button
                  loading={waiting}
                  disabled={!canSubmit}
                  onClick={shortenUrl}
                >
                  <ReloadIcon />Retry
                </Button>
              );

            case "ok":
              return (
                <Button
                  loading={waiting}
                  onClick={createCollection}
                >
                  <PlusIcon />Collection
                </Button>
              );
          }
        })()}

        {/* options button */}
        <Button variant={showOptions ? "solid" : "soft"} disabled={readOnly} onClick={toggleOptions}>
          <DotsHorizontalIcon />Options
        </Button>

        {/* reset button */}
        <Tooltip content="Reset">
          <IconButton color="red" radius="full" disabled={!(userUrl || linkAlias || linkPassword || resultUrl) || waiting} onClick={clearState}>
            <ResetIcon />
          </IconButton>
        </Tooltip>

      </Flex>

      {/* options */}
      {showOptions && (
        <Box
          style={{
            position: "absolute",
            top: "100%",
            left: 0,
            right: 0,
            marginTop: 8,
            zIndex: 20,
          }}
        >
          <Card size="2">
            <Flex direction="column" gap="2">
              <TextField.Root
                placeholder="Use a custom alias"
                readOnly={waiting}
                value={linkAlias}
                onChange={(e) => setLinkAlias(e.target.value)}
              />
              <TextField.Root
                placeholder="Set password"
                type="password"
                readOnly={waiting}
                value={linkPassword}
                onChange={(e) => setLinkPassword(e.target.value)}
              />
            </Flex>
          </Card>
        </Box>
      )}
    </Box>
  );
}