import { TextField, Box, IconButton, Button } from "@radix-ui/themes";
import { Link2Icon, DotsHorizontalIcon, ClipboardIcon, EraserIcon, PaperPlaneIcon, ReloadIcon } from "@radix-ui/react-icons"

import React from "react";
import { postJson } from "../api";
import { clipboardCopy } from "../util";

type ShortenRequest = {
  url: string;
  name?: string;
  password?: string;
}

type ShortenResponse = {
  alias: string;
}

type State = "idle" | "ok" | "err";

export function MainView() {

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
  };

  const submit = async () => {
    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);
    try {
      setWaiting(true);

      const body = { url: userUrl, name: urlName || undefined, password: userPassword || undefined } as ShortenRequest;
      const res = await postJson<ShortenRequest, ShortenResponse>("/api/shorten", body, ac.signal);
      const shortUrl = `${window.location.origin}/r/${res.alias}`;
      setResult(shortUrl);
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

  const inputStatus = state === "idle" ? "" : state === "ok" ? "ok" : "err";
  const firstButtonColor = state === "idle" ? "green" : state === "err" ? "green" : "indigo";
  const readOnly = waiting || state === "ok";
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
        <Button color={firstButtonColor} loading={waiting} onClick={async () => {
          switch (state) {
            case "idle": {
              await submit();
              setShowOptions(false);
              break;
            }
            case "ok": {
              await clipboardCopy(result);
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
            <EraserIcon />
          )}
        </Button>

        <Button color="red" disabled={!(userUrl || urlName || result) || waiting} onClick={() => {
          clearState();
        }}>
          <ClipboardIcon />
        </Button>
      </Box>
    </>
  );
}