import { Flex, TextField, Button, Box, IconButton } from "@radix-ui/themes";
import { Link2Icon, PaperPlaneIcon, ClipboardIcon, EraserIcon, ReloadIcon, DotsHorizontalIcon } from "@radix-ui/react-icons"
import { useAppController, type ViewEvents } from './controller';
import type { AppState } from './model';
import { useState } from "react";

type Button =
  | { type: "submit"; loading?: boolean }
  | { type: "retry"; loading?: boolean }
  | { type: "copy"; loading?: boolean }
  | { type: "clear"; loading?: boolean };

/**
 * Derives set of buttons to render from the app state.
 * @param state Current app state
 * @returns Buttons to render, in display order
 */
function getButtonsByState(state: AppState): Button[] {
  switch (state.result.kind) {
    case "none":
      return [{ type: "submit" }];

    case "waiting":
      return [
        { type: "submit", loading: true },
      ];

    case "copying":
      return [
        { type: "copy", loading: true },
        { type: "clear", loading: true },
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

function ActionButtons({ state, events, onCloseOptions, onClearOptions }: { state: AppState, events: ViewEvents, onCloseOptions: () => void, onClearOptions: () => void }) {
  return (
    <>
      {getButtonsByState(state).map((b) => {
        const key = b.type;

        switch (b.type) {
          case "submit":
            return (
              <Button key={key} onClick={() => {
                onCloseOptions();
                events.onSubmit();
              }} loading={b.loading} color="green">
                <PaperPlaneIcon />
              </Button>
            );

          case "retry":
            return (
              <Button key={key} onClick={() => {
                onCloseOptions();
                events.onRetry();
              }} loading={b.loading}>
                <ReloadIcon />
              </Button>
            );

          case "copy":
            return (
              <Button key={key} onClick={events.onCopy} loading={b.loading}>
                <ClipboardIcon />
              </Button>
            );

          case "clear":
            return (
              <Button key={key} onClick={() => {
                onClearOptions();
                events.onClear();
              }} loading={b.loading} color="red">
                <EraserIcon />
              </Button>
            );
        }
      })}
    </>
  );
}

function InputField({ state, events, showOptions, onToggleOptions }: { state: AppState; events: ViewEvents, showOptions: boolean, onToggleOptions: () => void }) {
  // Input field should be read-only if it's showing a result, during copying or waiting for request
  const isReadOnly = state.result.kind === "ok" || state.result.kind === "copying" || state.result.kind === "waiting";
  // Controls what value gets displayed to the user, the Request result or their input
  const value =
    state.result.kind === "ok" || state.result.kind === "copying"
      ? state.result.shortUrl
      : state.userUrl;

  const canToggleOptions = state.result.kind === "none" || state.result.kind === "err";

  return (
    <Box data-status={state.result.kind} className="inputField">
      <TextField.Root
        value={value}
        readOnly={isReadOnly}
        data-state={state.result.kind}
        style={{ width: "40rem", }}
        onChange={(e) => events.onUrlInput(e.target.value)
        }
      >
        <TextField.Slot><Link2Icon /></TextField.Slot>
        <TextField.Slot>
          <IconButton disabled={!canToggleOptions} variant="ghost" onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onToggleOptions();
          }}>
            <DotsHorizontalIcon />
          </IconButton>
        </TextField.Slot>
      </TextField.Root >
      {showOptions && (
        <Box>
          <TextField.Root
            placeholder="URL Name"
            value={state.userAlias}
            onChange={(e) => events.onAliasInput(e.target.value)}
          />
        </Box>
      )}
    </Box>
  );
}

export default function App() {
  const { state, events } = useAppController();

  const [showOptions, setShowOptions] = useState(false);

  const onToggleOptions = () => {
    setShowOptions(v => !v);
    events.onAliasInput("");
  }

  const onCloseOptions = () => {
    setShowOptions(false);
  }

  const onClearOptions = () => {
    setShowOptions(false);
    events.onAliasInput("");
  }

  return (
    <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
      <Flex gap="2" align="center">
        <InputField state={state} events={events} showOptions={showOptions} onToggleOptions={onToggleOptions} />
        <ActionButtons state={state} events={events} onCloseOptions={onCloseOptions} onClearOptions={onClearOptions} />
      </Flex>
    </Flex >
  );
}
