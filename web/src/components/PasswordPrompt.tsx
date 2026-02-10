import { TextField, Box, Button, Flex, Text } from "@radix-ui/themes";
import { LockClosedIcon } from "@radix-ui/react-icons"

import React from "react";

type State = "idle" | "ok" | "err";

export function PasswordPrompt({ alias }: { alias: string }) {

  const [waiting, setWaiting] = React.useState(false);
  const [state, setState] = React.useState<State>("idle");

  const onSubmit = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();

    setWaiting(true);

    try {
      const form = ev.currentTarget;
      const fd = new FormData(form);
      const password = String(fd.get("password") ?? "");

      const url = `/r/${encodeURIComponent(alias)}?password=${encodeURIComponent(password)}`;
      const res = await fetch(url, { redirect: "manual" });

      if (res.type === "opaqueredirect") {
        window.location.href = url;
        return;
      }

      setState("err");
      const errMsg = res.status === 401 ? "Wrong password" : `Unexpected (${res.status})`;
      console.log(errMsg);
    } catch (err) {
      setState("err");
      const errMsg = err instanceof Error ? err.message : "Network error";
      console.log(errMsg);
    } finally {
      setWaiting(false);
    }
  };

  const inputStatus = state === "idle" ? "" : "err";
  return (
    <>
      <form onSubmit={onSubmit}>
        <fieldset disabled={waiting} style={{ border: 0, padding: 0, margin: 0 }}>
          <Flex direction="column" gap="4" mt="4" align="center">
            <LockClosedIcon width="20" height="20" />
            <Text size="4" weight="bold">This link is password-protected</Text>

            <Box data-status={inputStatus} className="inputField">
              <label>
                <Text as="div" size="4" mb="1" weight="bold">
                  Password
                </Text>
                <TextField.Root
                  name="password"
                  type="password"
                  placeholder="Enter the link password"
                  style={{ width: "20rem" }}
                  required
                />
              </label>
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
