import { TextField, Box, IconButton, Button, Flex, SegmentedControl } from "@radix-ui/themes";
import { Link2Icon, DotsHorizontalIcon, ClipboardIcon, EraserIcon, PaperPlaneIcon, ReloadIcon } from "@radix-ui/react-icons"

import React from "react";
import { postReq } from "../api";
import { clipboardCopy } from "../util";
import { CollectionCreator } from "./CollectionCreator";

import { useNotify } from "./NotifyProvider";

type ShortenRequest = {
  url: string;
  name?: string;
  password?: string;
};

type ShortenResponse = {
  alias: string;
};

type State = "idle" | "ok" | "err";

type Mode = "single" | "collection";

export function MainView() {
  const [mode, setMode] = React.useState<Mode>("single");

  return (
    <Flex direction="column" gap="4" align="center">
      <SegmentedControl.Root value={mode} onValueChange={(v) => setMode(v as Mode)}>
        <SegmentedControl.Item value="single">Single Link</SegmentedControl.Item>
        <SegmentedControl.Item value="collection">Collection</SegmentedControl.Item>
      </SegmentedControl.Root>

      {mode === "single" ? <SingleLinkView /> : <CollectionCreator />}
    </Flex>
  );
}

function SingleLinkView() {
  const { notifyOk, notifyErr, notifyShort, dismiss } = useNotify();

  const [userUrl, setUserUrl] = React.useState("");
  const [urlName, setUrlName] = React.useState<string | undefined>(undefined);
  const [userPassword, setUserPassword] = React.useState<string | undefined>(undefined);
  const [result, setResult] = React.useState("");
  const [showOptions, setShowOptions] = React.useState(false);
  const [state, setState] = React.useState<State>("idle");

  const [waiting, setWaiting] = React.useState(false);

  const clearState = () => {
    setUserUrl("");
    setUrlName(undefined);
    setUserPassword(undefined);
    setResult("");
    setShowOptions(false);
    setState("idle");
    dismiss();
  };

  const submit = async () => {
    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);
    try {
      dismiss();
      setWaiting(true);

      const body = { url: userUrl, name: urlName || undefined, password: userPassword || undefined } as ShortenRequest;
      const res = await postReq<ShortenRequest, ShortenResponse>("/api/shorten", body, ac.signal);
      const shortUrl = `${window.location.origin}/r/${res.alias}`;
      setResult(shortUrl);
      setState("ok");
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

  const inputStatus = state === "idle" ? "" : state === "ok" ? "ok" : "err";
  const firstButtonColor = state === "idle" ? "green" : state === "err" ? "green" : "indigo";
  const readOnly = waiting || state === "ok";
  const canSubmit = userUrl.trim().length > 0;
  return (
    <>
      <Box data-status={inputStatus} className="inputField">
        <TextField.Root
          value={state === "ok" ? result : userUrl}
          readOnly={readOnly}
          style={{ width: "40rem", }}
          onChange={(ev) => setUserUrl(ev.target.value)
          }
        >
          <TextField.Slot><Link2Icon /></TextField.Slot>
          <TextField.Slot>
            <IconButton disabled={readOnly} variant="ghost" onClick={() => {
              setUrlName("");
              setShowOptions(!showOptions);
            }}>
              <DotsHorizontalIcon />
            </IconButton>
          </TextField.Slot>
        </TextField.Root >
        {showOptions && (
          <Box>
            <TextField.Root
              placeholder="Pick a short name for your URL"
              value={urlName}
              onChange={(ev) => setUrlName(ev.target.value)}
            />
            <TextField.Root
              placeholder="Password (optional)"
              type="password"
              value={userPassword}
              onChange={(ev) => setUserPassword(ev.target.value)}
              style={{ marginTop: "0.5rem" }}
            />
          </Box>
        )}
      </Box>
      <Box>
        <Button color={firstButtonColor} loading={waiting} disabled={!canSubmit} onClick={async () => {
          switch (state) {
            case "idle": {
              await submit();
              setShowOptions(false);
              break;
            }
            case "ok": {
              await clipboardCopy(result);
              notifyShort("Copied to clipboard!");
              break;
            }
            case "err": {
              await submit();
              setShowOptions(false);
              break;
            }
          }
        }}>
          {state === "idle" ? (
            <PaperPlaneIcon />
          ) : state === "err" ? (
            <ReloadIcon />
          ) : (
            <ClipboardIcon />
          )}
        </Button>

        <Button color="red" disabled={!(userUrl || urlName || result) || waiting} onClick={() => {
          clearState();
        }}>
          <EraserIcon />
        </Button>
      </Box >
    </>
  );
}
