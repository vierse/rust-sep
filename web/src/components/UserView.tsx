import { Button, Dialog, Flex, IconButton, Inset, Table, Text, TextField } from "@radix-ui/themes";
import { ClipboardIcon, PersonIcon } from "@radix-ui/react-icons";

import React from "react";
import { getJson, postJson } from "../api";
import { clipboardCopy } from "../util";

type AuthRequest = {
  username: string;
  password: string;
}

type AuthResponse = {
  username: string;
}

export function UserView() {
  const [open, setOpen] = React.useState(false);
  const [waiting, setWaiting] = React.useState(false);
  const [user, setUser] = React.useState("");

  const onSubmit = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();

    setWaiting(true);
    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);

    try {
      const form = ev.currentTarget;
      const fd = new FormData(form);

      const username = String(fd.get("username") ?? "");
      const password = String(fd.get("password") ?? "");

      const submitter = (ev.nativeEvent as SubmitEvent).submitter as HTMLButtonElement | null;
      const action = submitter?.value;

      const body = { username, password } as AuthRequest;

      const user =
        action === "register"
          ? await postJson<AuthRequest, AuthResponse>("/api/auth/register", body, ac.signal)
          : await postJson<AuthRequest, AuthResponse>("/api/auth/login", body, ac.signal);

      setUser(user.username);
      setOpen(false);
      form.reset();
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : "Login failed";
      console.log(errMsg)
    } finally {
      setWaiting(false);
      clearTimeout(timeoutId);
    }
  };

  return (
    <>
      <Dialog.Root open={open} onOpenChange={(next) => {
        if (waiting && next === false) return;
        setOpen(next);
      }}>
        <Dialog.Trigger>
          <Button>{user && <PersonIcon />}{user || "Login"}</Button>
        </Dialog.Trigger>
        <Dialog.Content maxWidth="450px" size="4">
          {user === "" ? (
            <>
              <form onSubmit={onSubmit}>
                <fieldset disabled={waiting} style={{ border: 0, padding: 0, margin: 0 }}>
                  <Flex direction="column" gap="4" mt="4">
                    <label>
                      <Text as="div" size="4" mb="1" weight="bold">
                        Username
                      </Text>
                      <TextField.Root
                        name="username"
                        placeholder="Enter your username"
                        required
                      />
                    </label>
                    <label>
                      <Text as="div" size="4" mb="1" weight="bold">
                        Password
                      </Text>
                      <TextField.Root
                        name="password"
                        type="password"
                        placeholder="Enter your password"
                        required
                      />
                    </label>
                  </Flex>

                  <Flex gap="4" mt="6" justify="end">
                    <Button variant="soft" color="blue" type="submit" value="register">
                      Create an account
                    </Button>
                    <Button color="green" type="submit" value="login">
                      Login
                    </Button>
                  </Flex>
                </fieldset>
              </form>
            </>
          ) : (
            <>
              <Dialog.Title>{user}</Dialog.Title>
              <LinksTable />
            </>
          )}
        </Dialog.Content>
      </Dialog.Root >
    </>
  );
}

type LinkItem = { alias: string; url: string };

function LinksTable() {
  const [links, setLinks] = React.useState<LinkItem[]>([]);
  const [loading, setLoading] = React.useState(true);

  React.useEffect(() => {
    (async () => {
      try {
        const ac = new AbortController();
        const data = await getJson<LinkItem[]>("/api/user/list", ac.signal);
        setLinks(data);
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const copyLink = async (link: LinkItem) => {
    const shortUrl = `${window.location.origin}/r/${link.alias}`;
    clipboardCopy(shortUrl);
  };

  return (
    <Inset side="x" my="5">
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.ColumnHeaderCell>Link</Table.ColumnHeaderCell>
            <Table.ColumnHeaderCell>Source</Table.ColumnHeaderCell>
            <Table.ColumnHeaderCell>Action</Table.ColumnHeaderCell>
          </Table.Row>
        </Table.Header>

        <Table.Body>
          {loading ? (
            <Table.Row>
              <Table.Cell>Loading…</Table.Cell>
            </Table.Row>
          ) : links.length === 0 ? (
            <Table.Row>
            </Table.Row>
          ) : (
            links.map((link) => (
              <Table.Row key={link.alias}>
                <Table.RowHeaderCell>{link.alias}</Table.RowHeaderCell>
                <Table.Cell>{link.url}</Table.Cell>
                <Table.Cell>
                  <Flex gap="2" align="center">
                    <IconButton
                      variant="ghost"
                      onClick={() => copyLink(link)}
                    >
                      <ClipboardIcon />
                    </IconButton>
                  </Flex>
                </Table.Cell>
              </Table.Row>
            ))
          )}
        </Table.Body>
      </Table.Root>
    </Inset>
  );
}