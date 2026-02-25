import { Box, Flex, Card, Text, Button, TextField, Spinner } from "@radix-ui/themes";
import { Link2Icon, PlusIcon, ClipboardIcon, ExternalLinkIcon } from "@radix-ui/react-icons";

import React from "react";
import { getReq, LockedError, postReq } from "../api";
import { clipboardCopy, urlWithAlias } from "../util";
import { useNotify } from "./NotifyProvider";

type CollectionResponse = {
  alias: string;
  items: CollectionItem[];
  edit: boolean;
};

type CollectionItem = {
  position: number;
  url: string;
  title?: string;
};

type AddUrlRequest = {
  url: string;
  title?: string;
};

export function CollectionView({ alias }: { alias: string }) {
  const { notifyShort } = useNotify();

  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const [items, setItems] = React.useState<CollectionItem[]>([]);
  const [canEdit, setCanEdit] = React.useState(false);

  const [showAddForm, setShowAddForm] = React.useState(false);
  const [waiting, setWaiting] = React.useState(false);
  const [newUrl, setNewUrl] = React.useState("");
  const [newTitle, setNewTitle] = React.useState("");

  const toggleShowAddForm = () => {
    setNewUrl("");
    setNewTitle("");
    setShowAddForm((v) => !v);
  };

  const load = React.useCallback(async () => {
    setError(null);
    try {
      const data = await getReq<CollectionResponse>(
        `/api/collection/${encodeURIComponent(alias)}/list`
      );
      setItems(data.items);
      setCanEdit(data.edit);
    } catch (err) {
      if (err instanceof LockedError) {
        window.location.assign(err.unlockUrl);
        return;
      }
      const msg = err instanceof Error ? err.message : "Failed to load collection";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, [alias]);

  React.useEffect(() => {
    void load();
  }, [load]);

  const addItem = async () => {
    const url = newUrl.trim();
    const title = newTitle.trim();

    if (!url) return;

    const ac = new AbortController();
    const timeoutId = setTimeout(() => ac.abort(), 5_000);

    try {
      setWaiting(true);
      setError(null);

      await postReq<AddUrlRequest>(
        `/api/collection/${encodeURIComponent(alias)}/add`,
        {
          url,
          title: title || undefined,
        },
        ac.signal
      );

      const data = await getReq<CollectionResponse>(
        `/api/collection/${encodeURIComponent(alias)}/list`
      );

      setItems(data.items);
      setCanEdit(data.edit);

      setNewUrl("");
      setNewTitle("");
      setShowAddForm(false);
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") {
        setError("Request timed out");
      } else {
        setError(err instanceof Error ? err.message : "Failed to add URL");
      }
    } finally {
      setWaiting(false);
      clearTimeout(timeoutId);
    }
  };

  const canAdd = newUrl.trim().length > 0;

  if (loading) {
    return (
      <Flex justify="center" align="center" style={{ width: "100%", minHeight: "10rem" }}>
        <Spinner />
      </Flex>
    );
  }

  return (
    <Flex
      direction="column"
      gap="3"
      style={{
        maxWidth: "40rem",
        width: "100%",
        margin: "0 auto",
        padding: "1rem",
      }}
    >
      {/* header */}
      <Card size="2">
        <Flex direction="column" gap="2">
          <Flex align="center" justify="between" gap="2" wrap="wrap">
            <Flex gap="3" align="center" wrap="wrap">
              <Box>
                <Text size="5" weight="bold">
                  Collection
                </Text>
                <Text size="2" color="gray">
                  /{alias}
                </Text>
              </Box>

              <Button
                variant="soft"
                onClick={() => {
                  clipboardCopy(urlWithAlias(alias));
                  notifyShort("Copied to clipboard");
                }}
              >
                <ClipboardIcon />
                Copy
              </Button>
            </Flex>

            {canEdit && (
              <Button
                color="green"
                variant="soft"
                onClick={toggleShowAddForm}
                disabled={showAddForm}
              >
                <PlusIcon />
                Add URL
              </Button>
            )}
          </Flex>

          {/* add URL form */}
          {canEdit && showAddForm && (
            <Card size="1">
              <Flex direction="column" gap="2">
                <TextField.Root
                  placeholder="Paste a URL"
                  value={newUrl}
                  onChange={(e) => setNewUrl(e.target.value)}
                  disabled={waiting}
                >
                  <TextField.Slot>
                    <Link2Icon />
                  </TextField.Slot>
                </TextField.Root>

                <TextField.Root
                  placeholder="Title (optional)"
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  disabled={waiting}
                />

                <Flex justify="end" gap="2">
                  <Button
                    variant="soft"
                    onClick={() => {
                      setShowAddForm(false);
                      setNewUrl("");
                      setNewTitle("");
                    }}
                    disabled={waiting}
                  >
                    Cancel
                  </Button>

                  <Button
                    color="green"
                    loading={waiting}
                    disabled={!canAdd || waiting}
                    onClick={addItem}
                  >
                    Add
                  </Button>
                </Flex>
              </Flex>
            </Card>
          )}
        </Flex>
      </Card>

      {error && (
        <Card size="2">
          <Text color="red">{error}</Text>
        </Card>
      )}

      {/* URL list */}
      {items
        .slice()
        .sort((a, b) => a.position - b.position)
        .map((item) => (
          <Card asChild key={`${item.position}-${item.url}`} size="2">
            <a
              href={item.url}
              target="_blank"
              rel="noreferrer noopener"
              title={item.url}
              style={{
                display: "block",
                color: "inherit",
                textDecoration: "none",
              }}
            >
              <Flex align="center" justify="between" gap="3">
                <Box style={{ minWidth: 0, flex: 1 }}>
                  <Flex direction="column" gap="1">
                    {item.title?.trim() ? (
                      <Text size="3" weight="medium">
                        {item.title.trim()}
                      </Text>
                    ) : null}

                    <Flex align="center" justify="between" gap="2" style={{ minWidth: 0 }}>
                      <Text
                        size="2"
                        color="gray"
                        style={{
                          minWidth: 0,
                          flex: 1,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {item.url}
                      </Text>

                      <Box style={{ flexShrink: 0, display: "flex", alignItems: "center" }}>
                        <ExternalLinkIcon />
                      </Box>
                    </Flex>
                  </Flex>
                </Box>
              </Flex>
            </a>
          </Card>
        ))}
    </Flex>
  );
}