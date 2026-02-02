import { TextField, Box, IconButton, Button } from "@radix-ui/themes";
import { Link2Icon, DotsHorizontalIcon, ClipboardIcon, EraserIcon, PaperPlaneIcon, ReloadIcon } from "@radix-ui/react-icons"

import { Ev, type AppState } from "../model";
import type { Dispatch } from "../controller";
import React from "react";

type Button =
  | { type: "submit"; loading?: boolean }
  | { type: "retry"; loading?: boolean }
  | { type: "clear"; loading?: boolean }
  | { type: "copy"; }

/**
 * Derives set of buttons to render from the app state.
 * @param state Current app state
 * @returns Buttons to render, in display order
 */
function getButtonsByState(state: AppState): Button[] {
  switch (state.result.kind) {
    case "none":
      return [
        { type: "submit" }
      ];

    case "inflight":
      return [
        { type: "submit", loading: true },
      ];

    case "ok":
      return [
        { type: "copy" },
        { type: "clear" },
      ];

    case "err":
      return [
        { type: "retry" },
        { type: "clear" },
      ];
  }
}

export function MainView({ state, dispatch }: { state: AppState, dispatch: Dispatch }) {

  const [userUrl, setUserUrl] = React.useState("");
  const [urlName, setUrlName] = React.useState("");
  const [showOptions, setShowOptions] = React.useState(false);

  const isReadOnly = state.result.kind === "ok" || state.result.kind === "inflight";

  return (
    <>
      <Box data-status={state.result.kind} className="inputField">
        <TextField.Root
          value={state.result.kind === "ok" ? state.result.shortUrl : userUrl}
          readOnly={isReadOnly}
          data-state={state.result.kind}
          style={{ width: "40rem", }}
          onChange={(e) => setUserUrl(e.target.value)
          }
        >
          <TextField.Slot><Link2Icon /></TextField.Slot>
          <TextField.Slot>
            <IconButton disabled={isReadOnly} variant="ghost" onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
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
              onChange={(e) => setUrlName(e.target.value)}
            />
          </Box>
        )}
      </Box>
      <>
        {getButtonsByState(state).map((b) => {
          const key = b.type;

          switch (b.type) {
            case "submit":
              return (
                <Button key={key} onClick={() => {
                  setShowOptions(false);
                  dispatch(Ev.submit(userUrl, urlName));
                }} loading={b.loading} color="green">
                  <PaperPlaneIcon />
                </Button>
              );

            case "retry":
              return (
                <Button key={key} onClick={() => {
                  setShowOptions(false);
                  dispatch(Ev.submit(userUrl, urlName));
                }} loading={b.loading}>
                  <ReloadIcon />
                </Button>
              );

            case "clear":
              return (
                <Button key={key} onClick={() => {
                  setUserUrl("");
                  setUrlName("");
                  setShowOptions(false);

                  dispatch(Ev.clear());
                }} loading={b.loading} color="red">
                  <EraserIcon />
                </Button>
              );

            case "copy":
              return (
                <Button key={key} onClick={() => {
                  dispatch(Ev.copy());
                }}>
                  <ClipboardIcon />
                </Button>
              );
          }
        })}
      </>
    </>
  );
}