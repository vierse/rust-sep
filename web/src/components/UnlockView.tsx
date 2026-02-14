import { TextField, Box, Button, Flex, Text } from "@radix-ui/themes";
import { LockClosedIcon } from "@radix-ui/react-icons"

import React from "react";
import { postReq } from "../api";
import { useNotify } from "./NotifyProvider";

type State = "idle" | "err";

type UnlockRequest = {
  password: string;
};

type UnlockResponse = {
  url: string;
};

export function UnlockView({ alias }: { alias: string }) {

  const [waiting, setWaiting] = React.useState(false);
  const [state, setState] = React.useState<State>("idle");

  const { notifyErr } = useNotify();

  const onSubmit = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();

    setState("idle");
    setWaiting(true);

    const form = ev.currentTarget;
    const fd = new FormData(form);

    const password = String(fd.get("password") ?? "");

    const body = { password } as UnlockRequest;
    const path = `/api/unlock/${encodeURIComponent(alias)}`;

    try {
      const res = await postReq<UnlockRequest, UnlockResponse>(path, body);
      window.location.assign(res.url);
    } catch (err) {
      setState("err");
      const errMsg = err instanceof Error ? err.message : "Internal error";
      notifyErr("Could not unlock the link", errMsg);
      setWaiting(false);
    }
  }

  const inputStatus = state === "idle" ? "" : "err";
  return (
    <>
      <form onSubmit={onSubmit}>
        <fieldset disabled={waiting} style={{ border: 0, padding: 0, margin: 0 }}>
          <Flex direction="column" gap="4" mt="4" align="center">
            <LockClosedIcon width="20" height="20" />
            <Text size="4" weight="bold">This link is password-protected</Text>

            <Box data-status={inputStatus} className="inputField">
              <TextField.Root
                name="password"
                type="password"
                placeholder="Enter the link password"
                style={{ width: "20rem" }}
                required
              />
            </Box>

            <Flex gap="4" mt="2" justify="end">
              <Button color="green" type="submit" loading={waiting}>
                Unlock
              </Button>
            </Flex>
          </Flex>
        </fieldset>
      </form>
    </>
  );
}
