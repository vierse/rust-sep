import { Flex, TextField, Button, Box } from "@radix-ui/themes";
import { Link2Icon, PaperPlaneIcon, ClipboardIcon, EraserIcon, ReloadIcon } from "@radix-ui/react-icons"
import { useAppController, type ViewEvents } from './controller';
import type { AppState } from './model';

type Button =
  | { type: "submit"; loading?: boolean }
  | { type: "retry"; loading?: boolean }
  | { type: "copy"; loading?: boolean }
  | { type: "clear"; loading?: boolean };

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

function ActionButtons({ state, events }: { state: AppState, events: ViewEvents }) {
  return (
    <>
      {getButtonsByState(state).map((b) => {
        const key = b.type;

        switch (b.type) {
          case "submit":
            return (
              <Button key={key} onClick={events.onSubmit} loading={b.loading} color="green">
                <PaperPlaneIcon />
              </Button>
            );

          case "retry":
            return (
              <Button key={key} onClick={events.onRetry} loading={b.loading}>
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
              <Button key={key} onClick={events.onClear} loading={b.loading} color="red">
                <EraserIcon />
              </Button>
            );
        }
      })}
    </>
  );
}

function InputField({ state, events }: { state: AppState; events: ViewEvents }) {
  const isReadOnly = state.result.kind === "ok" || state.result.kind === "copying" || state.result.kind === "waiting";
  const value =
    state.result.kind === "ok" || state.result.kind === "copying"
      ? state.result.shortUrl
      : state.userInput;

  return (
    <Box data-status={state.result.kind} className="inputField">
      <TextField.Root
        value={value}
        readOnly={isReadOnly}
        data-state={state.result.kind}
        style={{ width: "40rem", }}
        onChange={(e) => events.onInput(e.target.value)
        }
      >
        <TextField.Slot><Link2Icon /></TextField.Slot>
      </TextField.Root >
    </Box>
  );
}

export default function App() {
  const { state, events } = useAppController();

  return (
    <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
      <Flex gap="2" align="center">
        <InputField state={state} events={events} />
        <ActionButtons state={state} events={events} />
      </Flex>
    </Flex >
  );
}
