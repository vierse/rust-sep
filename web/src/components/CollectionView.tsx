import { Flex, Text, Box } from "@radix-ui/themes";
import { Link2Icon } from "@radix-ui/react-icons";

import React from "react";
import { getReq } from "../api";

type CollectionItem = {
  url: string;
  position: number;
};

export function CollectionView({ alias }: { alias: string }) {
  const [items, setItems] = React.useState<CollectionItem[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState("");

  React.useEffect(() => {
    (async () => {
      try {
        const data = await getReq<CollectionItem[]>(
          `/api/collection/${encodeURIComponent(alias)}`
        );
        setItems(data);
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to load collection";
        setError(msg);
      } finally {
        setLoading(false);
      }
    })();
  }, [alias]);

  if (loading) {
    return <Text size="3">Loading…</Text>;
  }

  if (error) {
    return <Text size="3" color="red">{error}</Text>;
  }

  return (
    <Flex direction="column" gap="3" style={{ maxWidth: "40rem", width: "100%" }}>
      <Text size="4" weight="bold">{alias}</Text>
      {items.map((item) => (
        <Box key={item.position}>
          <Flex gap="2" align="center">
            <Link2Icon />
            <a href={item.url} target="_blank" rel="noopener noreferrer">
              <Text size="2">{item.url}</Text>
            </a>
          </Flex>
        </Box>
      ))}
    </Flex>
  );
}
