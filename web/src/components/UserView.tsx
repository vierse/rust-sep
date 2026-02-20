import { Button, Dialog, Flex, IconButton, Inset, Separator, Table, Text, TextField } from "@radix-ui/themes";
import { ClipboardIcon, Cross1Icon, PersonIcon } from "@radix-ui/react-icons";

import React from "react";
import { deleteReq, getReq, postEmpty, postReq } from "../api";
import { clipboardCopy } from "../util";
import { useNotify } from "./NotifyProvider";

type AuthRequest = {
  username: string;
  password: string;
};

type AuthResponse = {
  username: string;
};

export function UserView() {
  const [open, setOpen] = React.useState(false);
  const [waiting, setWaiting] = React.useState(false);
  const [user, setUser] = React.useState("");

  const { notifyOk, notifyErr, dismiss } = useNotify();

  React.useEffect(() => {
    (async () => {
      try {
        const user = await getReq<AuthResponse>("/api/auth/me");
        setUser(user.username);
        notifyOk("Restored previous session");
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : "Session restore failed";
        console.log(errMsg);
      }
    })();
  }, []);

  const onSubmit = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();

    setWaiting(true);

    const form = ev.currentTarget;
    const fd = new FormData(form);

    const username = String(fd.get("username") ?? "");
    const password = String(fd.get("password") ?? "");

    const submitter = (ev.nativeEvent as SubmitEvent).submitter as HTMLButtonElement | null;
    const action = submitter?.value;

    const body = { username, password } as AuthRequest;

    try {
      const user =
        action === "register"
          ? await postReq<AuthRequest, AuthResponse>("/api/auth/register", body)
          : await postReq<AuthRequest, AuthResponse>("/api/auth/login", body);

      setOpen(false);
      setUser(user.username);
      form.reset();

      notifyOk("Logged in!");
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : "Internal error";
      const notifyReason = action === "register" ? "Could not create an account" : "Could not login";
      notifyErr(notifyReason, errMsg);
    } finally {
      setWaiting(false);
    }
  };

  const onLogout = async () => {
    try {
      setWaiting(true);
      await postEmpty("/api/user/logout");
      notifyOk("Logged out");
      setUser("");
      setOpen(false);
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : "Internal error";
      notifyErr("Could not logout", errMsg);
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
          <Button onClick={() => { dismiss() }}>{user && <PersonIcon />}{user || "Login"}</Button>
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
              <Flex align="center" justify="between">
                <Dialog.Title>{user}</Dialog.Title>
                <Button loading={waiting} color="red" onClick={onLogout}>Logout</Button>
              </Flex>
              <Text size="2" weight="bold" as="div" mb="1">Links</Text>
              {open && <LinksTable />}
              <Separator size="4" my="4" />
              <Text size="2" weight="bold" as="div" mb="1">Collections</Text>
              {open && <CollectionsTable />}
            </>
          )}
        </Dialog.Content>
      </Dialog.Root >
    </>
  );
}

type CollectionListItem = { alias: string; item_count: number };

function CollectionsTable() {
  const [collections, setCollections] = React.useState<CollectionListItem[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [removing, setRemoving] = React.useState(false);

  React.useEffect(() => {
    (async () => {
      try {
        const data = await getReq<CollectionListItem[]>("/api/user/collections");
        setCollections(data);
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const copyCollection = async (c: CollectionListItem) => {
    const url = `${window.location.origin}/collection/${encodeURIComponent(c.alias)}`;
    clipboardCopy(url);
  };

  const removeCollection = async (c: CollectionListItem) => {
    setRemoving(true);
    try {
      await deleteReq(`/api/user/collection/${encodeURIComponent(c.alias)}`);
      setCollections((xs) => xs.filter((x) => x.alias !== c.alias));
    } catch (err) {
      console.log(err);
    } finally {
      setRemoving(false);
    }
  };

  return (
    <Inset side="x">
      <Table.Root size="1">
        <Table.Header>
          <Table.Row>
            <Table.ColumnHeaderCell>Alias</Table.ColumnHeaderCell>
            <Table.ColumnHeaderCell>Items</Table.ColumnHeaderCell>
            <Table.ColumnHeaderCell>Action</Table.ColumnHeaderCell>
          </Table.Row>
        </Table.Header>

        <Table.Body>
          {loading ? (
            <Table.Row>
              <Table.Cell>Loading…</Table.Cell>
            </Table.Row>
          ) : collections.length === 0 ? (
            <Table.Row>
            </Table.Row>
          ) : (
            collections.map((c) => (
              <Table.Row key={c.alias}>
                <Table.RowHeaderCell>{c.alias}</Table.RowHeaderCell>
                <Table.Cell>{c.item_count}</Table.Cell>
                <Table.Cell>
                  <Flex gap="2" align="center">
                    <IconButton
                      variant="ghost"
                      onClick={() => copyCollection(c)}
                    >
                      <ClipboardIcon />
                    </IconButton>

                    <IconButton disabled={removing} variant="ghost" onClick={() => removeCollection(c)}>
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

type LinkItem = {
  alias: string;
  url: string
};

function LinksTable() {
  const [links, setLinks] = React.useState<LinkItem[]>([]);
  const [loading, setLoading] = React.useState(true);

  const [removingLink, setRemovingLink] = React.useState(false);

  const { notifyOk, notifyErr, notifyShort } = useNotify();

  React.useEffect(() => {
    (async () => {
      try {
        const data = await getReq<LinkItem[]>("/api/user/list");
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
    notifyShort("Copied to clipboard!");
  };

  const removeLink = async (link: LinkItem) => {
    setRemovingLink(true);
    try {
      await deleteReq(`/api/user/link/${encodeURIComponent(link.alias)}`);
      setLinks((xs) => xs.filter((l) => l.alias !== link.alias));
      notifyOk("Link successfully deleted");
    } catch (err) {
      notifyErr("Failed to delete the link");
      console.log(err);
    } finally {
      setRemovingLink(false);
    }
  };

  return (
    <Inset side="x">
      <Table.Root size="1">
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
