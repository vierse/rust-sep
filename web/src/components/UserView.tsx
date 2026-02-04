import { Button, Dialog, Flex, IconButton, Inset, Table, Text, TextField } from "@radix-ui/themes";
import { ClipboardIcon, Cross1Icon, PersonIcon } from "@radix-ui/react-icons";

import React from "react";
import { deleteReq, getJson, postJson } from "../api";
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

  React.useEffect(() => {
    (async () => {
      try {
        const user = await getJson<AuthResponse>("/api/auth/me");
        setUser(user.username);
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : "Session restore failed";
        console.log(errMsg)
      }
    })();
  }, []);
  const onSubmit = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();

    setWaiting(true);

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
          ? await postJson<AuthRequest, AuthResponse>("/api/auth/register", body)
          : await postJson<AuthRequest, AuthResponse>("/api/auth/login", body);

      setUser(user.username);
      setOpen(false);
      form.reset();
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : "Login failed";
      console.log(errMsg)
    } finally {
      setWaiting(false);
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

  const [removingLink, setRemovingLink] = React.useState(false);

  React.useEffect(() => {
    (async () => {
      try {
        const data = await getJson<LinkItem[]>("/api/user/list");
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

  const removeLink = async (link: LinkItem) => {
    setRemovingLink(true);
    try {
      await deleteReq(`/api/user/link/${encodeURIComponent(link.alias)}`);
      setLinks((xs) => xs.filter((l) => l.alias !== link.alias));
    } catch (err) {
      console.log(err);
    } finally {
      setRemovingLink(false);
    }
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

                    <IconButton disabled={removingLink} variant="ghost" onClick={() => removeLink(link)}>
                      <Cross1Icon />
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